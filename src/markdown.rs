use regex::Regex;

use crate::error::{Arxiv2MdError, Result};
use crate::model::{
    ArxivId, CliOptions, ConversionOutput, Frontmatter, PaperDocument, PaperMetadata,
    SectionFilterMode, SectionNode,
};

pub fn parse_markdown_sections(markdown: &str) -> Vec<SectionNode> {
    let heading_re = Regex::new(r"^(#{1,6})\s+(.+?)\s*$").expect("valid regex");
    let mut parsed = Vec::new();
    let mut current: Option<(usize, String, Vec<String>)> = None;

    for line in markdown.lines() {
        if let Some(captures) = heading_re.captures(line) {
            if let Some((level, title, lines)) = current.take() {
                parsed.push((level as u8, title, lines.join("\n").trim().to_owned()));
            }
            let level = captures
                .get(1)
                .map(|value| value.as_str().len())
                .unwrap_or(1);
            let title = captures
                .get(2)
                .map(|value| value.as_str().trim().to_owned())
                .unwrap_or_default();
            current = Some((level, title, Vec::new()));
        } else if let Some((_, _, lines)) = current.as_mut() {
            lines.push(line.to_owned());
        }
    }

    if let Some((level, title, lines)) = current.take() {
        parsed.push((level as u8, title, lines.join("\n").trim().to_owned()));
    }

    build_tree(parsed)
}

pub fn cleanup_generated_markdown(markdown: &str) -> String {
    let mut output = markdown.replace("\r\n", "\n");
    let figure_re = Regex::new(r"(?is)<figure.*?</figure>").expect("valid regex");
    let html_tag_re = Regex::new(r"(?is)<[^>]+>").expect("valid regex");
    output = figure_re.replace_all(&output, "").into_owned();
    output = html_tag_re.replace_all(&output, "").into_owned();
    output = normalize_display_math(&output);
    collapse_blank_lines(&output)
}

pub fn render(
    id: &ArxivId,
    options: &CliOptions,
    mut document: PaperDocument,
    warnings: Vec<String>,
) -> Result<ConversionOutput> {
    if !options.keep_refs {
        document.sections = filter_sections(
            &document.sections,
            &["references", "bibliography"],
            SectionFilterMode::Exclude,
        );
    }

    if !options.sections.is_empty() && document.supports_tree {
        document.sections = filter_sections(
            &document.sections,
            &options.sections,
            options.section_filter_mode.clone(),
        );
    }

    let markdown = if let Some(body) = document.fallback_markdown.take() {
        render_fallback_document(
            id,
            options,
            &document.metadata,
            &body,
            document.supports_tree,
            document.resolved_via.clone(),
        )
    } else {
        render_sectioned_document(
            id,
            options,
            &document.metadata,
            &document.sections,
            document.supports_tree,
            document.resolved_via.clone(),
        )?
    };

    Ok(ConversionOutput { markdown, warnings })
}

fn render_sectioned_document(
    id: &ArxivId,
    options: &CliOptions,
    metadata: &PaperMetadata,
    sections: &[SectionNode],
    supports_tree: bool,
    resolved_via: crate::model::ResolvedVia,
) -> Result<String> {
    let mut blocks = Vec::new();
    if options.frontmatter {
        blocks.push(generate_frontmatter(id, metadata, resolved_via)?);
    }

    if let Some(title) = &metadata.title {
        blocks.push(format!("# {title}"));
    }
    if let Some(abstract_text) = &metadata.abstract_text {
        blocks.push("## Abstract".to_owned());
        blocks.push(abstract_text.trim().to_owned());
    }
    if options.keep_toc && supports_tree {
        let toc = render_toc(sections, 0);
        if !toc.is_empty() {
            blocks.push("## Contents".to_owned());
            blocks.push(toc);
        }
    }
    if options.include_tree && supports_tree {
        let tree = render_tree(sections, 0);
        if !tree.is_empty() {
            blocks.push("## Sections".to_owned());
            blocks.push(tree);
        }
    }
    for section in sections {
        blocks.extend(render_section(section));
    }

    Ok(collapse_blank_lines(&blocks.join("\n\n")))
}

fn render_fallback_document(
    id: &ArxivId,
    options: &CliOptions,
    metadata: &PaperMetadata,
    body: &str,
    _supports_tree: bool,
    resolved_via: crate::model::ResolvedVia,
) -> String {
    let mut blocks = Vec::new();
    if options.frontmatter {
        if let Ok(frontmatter) = generate_frontmatter(id, metadata, resolved_via) {
            blocks.push(frontmatter);
        }
    }
    if let Some(title) = &metadata.title {
        blocks.push(format!("# {title}"));
    }
    if let Some(abstract_text) = &metadata.abstract_text {
        blocks.push("## Abstract".to_owned());
        blocks.push(abstract_text.trim().to_owned());
    }
    blocks.push(body.trim().to_owned());
    collapse_blank_lines(&blocks.join("\n\n"))
}

fn generate_frontmatter(
    id: &ArxivId,
    metadata: &PaperMetadata,
    resolved_via: crate::model::ResolvedVia,
) -> Result<String> {
    let frontmatter = Frontmatter {
        title: metadata.title.clone(),
        arxiv_id: id.normalized.clone(),
        version: id.version.clone(),
        authors: metadata.authors.clone(),
        categories: metadata.categories.clone(),
        published: metadata.published.clone(),
        updated: metadata.updated.clone(),
        abs_url: id.abs_url.clone(),
        resolved_via,
    };
    let mut yaml = serde_yaml::to_string(&frontmatter)
        .map_err(|error| Arxiv2MdError::Markdown(error.to_string()))?;
    if !yaml.starts_with("---") {
        yaml = format!("---\n{}\n---", yaml.trim_end());
    }
    Ok(yaml.trim_end().to_owned())
}

fn render_section(section: &SectionNode) -> Vec<String> {
    let mut blocks = Vec::new();
    let level = section.level.clamp(2, 6);
    blocks.push(format!(
        "{} {}",
        "#".repeat(level as usize),
        section.title.trim()
    ));
    if !section.markdown.trim().is_empty() {
        blocks.push(section.markdown.trim().to_owned());
    }
    for child in &section.children {
        blocks.extend(render_section(child));
    }
    blocks
}

fn render_toc(sections: &[SectionNode], indent: usize) -> String {
    let mut lines = Vec::new();
    for section in sections {
        lines.push(format!("{}- {}", "  ".repeat(indent), section.title));
        let nested = render_toc(&section.children, indent + 1);
        if !nested.is_empty() {
            lines.push(nested);
        }
    }
    lines.join("\n")
}

fn render_tree(sections: &[SectionNode], indent: usize) -> String {
    let mut lines = Vec::new();
    for section in sections {
        lines.push(format!("{}- {}", "  ".repeat(indent), section.title));
        let nested = render_tree(&section.children, indent + 1);
        if !nested.is_empty() {
            lines.push(nested);
        }
    }
    lines.join("\n")
}

pub fn remove_references_from_text(body: &str) -> String {
    let re = Regex::new(r"(?im)^\s*(references|bibliography)\s*$").expect("valid regex");
    if let Some(found) = re.find(body) {
        body[..found.start()].trim_end().to_owned()
    } else {
        body.trim().to_owned()
    }
}

fn normalize_section_title(title: &str) -> String {
    let prefix_re = Regex::new(r"^[\dA-Za-z.\-]+\s+").expect("valid regex");
    let lower = title.trim().to_lowercase();
    let stripped = prefix_re.replace(&lower, "");
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn filter_sections(
    sections: &[SectionNode],
    selected: &[impl AsRef<str>],
    mode: SectionFilterMode,
) -> Vec<SectionNode> {
    let selected_titles = selected
        .iter()
        .map(|title| normalize_section_title(title.as_ref()))
        .collect::<Vec<_>>();
    if selected_titles.is_empty() {
        return sections.to_owned();
    }
    sections
        .iter()
        .filter_map(|section| filter_section(section, &selected_titles, &mode))
        .collect()
}

fn filter_section(
    section: &SectionNode,
    selected: &[String],
    mode: &SectionFilterMode,
) -> Option<SectionNode> {
    let title = normalize_section_title(&section.title);
    let matches = selected.iter().any(|selected| selected == &title);
    let filtered_children = section
        .children
        .iter()
        .filter_map(|child| filter_section(child, selected, mode))
        .collect::<Vec<_>>();

    match mode {
        SectionFilterMode::Include => {
            if matches || !filtered_children.is_empty() {
                Some(SectionNode {
                    title: section.title.clone(),
                    level: section.level,
                    anchor: section.anchor.clone(),
                    markdown: section.markdown.clone(),
                    children: filtered_children,
                })
            } else {
                None
            }
        }
        SectionFilterMode::Exclude => {
            if matches {
                None
            } else {
                Some(SectionNode {
                    title: section.title.clone(),
                    level: section.level,
                    anchor: section.anchor.clone(),
                    markdown: section.markdown.clone(),
                    children: filtered_children,
                })
            }
        }
    }
}

fn build_tree(parsed: Vec<(u8, String, String)>) -> Vec<SectionNode> {
    #[derive(Clone)]
    struct TempNode {
        title: String,
        level: u8,
        markdown: String,
        children: Vec<usize>,
    }

    let mut nodes = Vec::<TempNode>::new();
    let mut roots = Vec::<usize>::new();
    let mut stack = Vec::<usize>::new();

    for (level, title, markdown) in parsed {
        let index = nodes.len();
        nodes.push(TempNode {
            title,
            level,
            markdown,
            children: Vec::new(),
        });
        while let Some(last) = stack.last().copied() {
            if nodes[last].level >= level {
                stack.pop();
            } else {
                break;
            }
        }
        if let Some(parent) = stack.last().copied() {
            nodes[parent].children.push(index);
        } else {
            roots.push(index);
        }
        stack.push(index);
    }

    fn materialize(index: usize, nodes: &[TempNode]) -> SectionNode {
        let node = &nodes[index];
        SectionNode {
            title: node.title.clone(),
            level: node.level,
            anchor: None,
            markdown: node.markdown.clone(),
            children: node
                .children
                .iter()
                .map(|child| materialize(*child, nodes))
                .collect(),
        }
    }

    roots
        .into_iter()
        .map(|index| materialize(index, &nodes))
        .collect()
}

fn normalize_display_math(input: &str) -> String {
    let re = Regex::new(r"(?s)\$\$(.+?)\$\$").expect("valid regex");
    let mut output = input.to_owned();
    for found in re.find_iter(input).collect::<Vec<_>>().into_iter().rev() {
        let body = &input[found.start() + 2..found.end() - 2];
        let before = &input[..found.start()];
        let after = &input[found.end()..];
        let line_start_ok = before.is_empty() || before.ends_with('\n');
        let line_end_ok = after.is_empty() || after.starts_with('\n');
        if line_start_ok && line_end_ok {
            continue;
        }
        let replacement = format!("\n$${}$$\n", body.trim());
        output.replace_range(found.start()..found.end(), &replacement);
    }
    output
}

pub fn collapse_blank_lines(input: &str) -> String {
    let re = Regex::new(r"\n{3,}").expect("valid regex");
    re.replace_all(input.trim(), "\n\n").into_owned()
}

#[cfg(test)]
mod tests {
    use crate::model::{
        CliOptions, PaperDocument, PaperMetadata, ResolvedVia, SectionFilterMode, SectionNode,
    };

    use super::{parse_markdown_sections, remove_references_from_text, render};

    fn options() -> CliOptions {
        CliOptions {
            input: "1706.03762".into(),
            output: "-".into(),
            frontmatter: true,
            keep_refs: false,
            keep_toc: false,
            remove_inline_citations: false,
            include_tree: false,
            refresh: false,
            sections: vec![],
            section_filter_mode: SectionFilterMode::Exclude,
            cache_dir: None,
            pandoc_path: "pandoc".into(),
        }
    }

    #[test]
    fn parses_markdown_heading_tree() {
        let sections =
            parse_markdown_sections("## Intro\nHello\n### Background\nWorld\n## Methods\nDone");
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].title, "Intro");
        assert_eq!(sections[0].children[0].title, "Background");
        assert_eq!(sections[1].title, "Methods");
    }

    #[test]
    fn strips_references_tail() {
        let output = remove_references_from_text("## Intro\nBody\n\nReferences\n[1] test");
        assert_eq!(output, "## Intro\nBody");
    }

    #[test]
    fn renders_frontmatter_and_filters_refs() {
        let id = crate::id::parse("1706.03762").expect("valid id");
        let document = PaperDocument {
            metadata: PaperMetadata {
                title: Some("Attention Is All You Need".into()),
                authors: vec!["A".into(), "B".into()],
                abstract_text: Some("Abstract text".into()),
                categories: vec!["cs.CL".into()],
                published: Some("2017-06-12T17:57:26Z".into()),
                updated: Some("2017-12-05T17:57:26Z".into()),
            },
            sections: vec![
                SectionNode {
                    title: "1 Introduction".into(),
                    level: 2,
                    anchor: None,
                    markdown: "Intro body".into(),
                    children: vec![],
                },
                SectionNode {
                    title: "References".into(),
                    level: 2,
                    anchor: None,
                    markdown: "[1] Ref".into(),
                    children: vec![],
                },
            ],
            fallback_markdown: None,
            resolved_via: ResolvedVia::ArxivHtml,
            supports_tree: true,
        };

        let rendered = render(&id, &options(), document, vec![]).expect("render succeeds");
        assert!(rendered.markdown.contains("resolved_via: arxiv_html"));
        assert!(rendered.markdown.contains("# Attention Is All You Need"));
        assert!(rendered.markdown.contains("## Abstract"));
        assert!(rendered.markdown.contains("## 1 Introduction"));
        assert!(!rendered.markdown.contains("References"));
    }
}
