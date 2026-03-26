use ego_tree::NodeRef;
use regex::Regex;
use scraper::{ElementRef, Html, Selector, node::Node};

use crate::error::{Arxiv2MdError, Result};
use crate::model::{HtmlPaper, SectionNode};

pub fn parse_document(body: &str, remove_inline_citations: bool) -> Result<HtmlPaper> {
    let document = Html::parse_document(body);
    let Some(root) = document_root(&document) else {
        return Err(Arxiv2MdError::Html("missing article/body root".into()));
    };

    if !looks_like_arxiv_html(&document) {
        return Err(Arxiv2MdError::Html(
            "page does not look like arXiv HTML".into(),
        ));
    }

    let title = select_first_text(&document, "h1.ltx_title_document, h1.ltx_title");
    let authors = extract_authors(&document);
    let abstract_text = select_first_text(&document, ".ltx_abstract");
    let sections = extract_sections(root, remove_inline_citations)?;

    Ok(HtmlPaper {
        title,
        authors,
        abstract_text,
        sections,
    })
}

fn document_root<'a>(document: &'a Html) -> Option<ElementRef<'a>> {
    select_first(document, "article.ltx_document")
        .or_else(|| select_first(document, "article"))
        .or_else(|| select_first(document, "body"))
}

fn looks_like_arxiv_html(document: &Html) -> bool {
    select_first(document, "article.ltx_document").is_some()
        || select_first(document, ".ltx_abstract").is_some()
        || select_first(document, ".ltx_section").is_some()
}

fn extract_authors(document: &Html) -> Vec<String> {
    let Some(container) = select_first(document, "div.ltx_authors") else {
        return Vec::new();
    };
    let span_selector = selector("span");
    let mut authors = Vec::new();
    for span in container.select(&span_selector) {
        for candidate in clean_author_text(&span.text().collect::<Vec<_>>().join(" ")) {
            if !authors.iter().any(|author| author == &candidate) {
                authors.push(candidate);
            }
        }
    }
    authors
}

fn extract_sections(
    root: ElementRef<'_>,
    remove_inline_citations: bool,
) -> Result<Vec<SectionNode>> {
    let heading_selector = selector("h1, h2, h3, h4, h5, h6");
    let mut flat = Vec::new();
    for heading in root.select(&heading_selector) {
        let name = heading.value().name();
        let level = name.trim_start_matches('h').parse::<u8>().unwrap_or(2);
        let classes = heading.value().classes().collect::<Vec<_>>();
        if classes.contains(&"ltx_title_document") {
            continue;
        }
        if has_ancestor(&heading, "nav") || has_ancestor_class(&heading, "ltx_abstract") {
            continue;
        }
        let title = normalize_text(&heading.text().collect::<Vec<_>>().join(" "));
        if title.is_empty() {
            continue;
        }
        let fragment = collect_section_fragment(&heading);
        let markdown = convert_fragment_to_markdown(&fragment, remove_inline_citations)?;
        flat.push((
            level,
            title,
            heading.value().attr("id").map(ToOwned::to_owned),
            markdown,
        ));
    }
    Ok(build_tree(flat))
}

fn convert_fragment_to_markdown(html: &str, remove_inline_citations: bool) -> Result<String> {
    let fragment = Html::parse_fragment(html);
    let mut blocks = Vec::new();
    for child in fragment.tree.root().children() {
        blocks.extend(serialize_node(
            child.value(),
            &child,
            remove_inline_citations,
        )?);
    }
    Ok(crate::markdown::collapse_blank_lines(&blocks.join("\n\n")))
}

fn serialize_node(
    node: &Node,
    handle: &NodeRef<'_, Node>,
    remove_inline_citations: bool,
) -> Result<Vec<String>> {
    match node {
        Node::Text(text) => {
            let cleaned = normalize_text(text);
            if cleaned.is_empty() {
                Ok(Vec::new())
            } else {
                Ok(vec![cleaned])
            }
        }
        Node::Element(_) => {
            let Some(element) = ElementRef::wrap(*handle) else {
                return Ok(Vec::new());
            };
            let name = element.value().name();
            if matches!(
                name,
                "script" | "style" | "nav" | "noscript" | "meta" | "link"
            ) {
                return Ok(Vec::new());
            }

            match name {
                "section" | "article" | "div" | "span" => {
                    serialize_children(&element, remove_inline_citations)
                }
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    let level = name
                        .trim_start_matches('h')
                        .parse::<usize>()
                        .unwrap_or(2)
                        .clamp(2, 6);
                    let title = normalize_text(&element.text().collect::<Vec<_>>().join(" "));
                    if title.is_empty() {
                        Ok(Vec::new())
                    } else {
                        Ok(vec![format!("{} {}", "#".repeat(level), title)])
                    }
                }
                "p" => {
                    let text = serialize_inline_children(&element, remove_inline_citations)?;
                    if text.is_empty() {
                        Ok(Vec::new())
                    } else {
                        Ok(vec![text])
                    }
                }
                "ul" | "ol" => {
                    let lines = serialize_list(&element, 0, remove_inline_citations)?;
                    if lines.is_empty() {
                        Ok(Vec::new())
                    } else {
                        Ok(vec![lines.join("\n")])
                    }
                }
                "figure" => {
                    let mut blocks = serialize_children(&element, remove_inline_citations)?;
                    blocks.retain(|block| !block.trim().is_empty());
                    Ok(blocks)
                }
                "table" => {
                    let table = serialize_table(&element, remove_inline_citations)?;
                    if table.is_empty() {
                        Ok(Vec::new())
                    } else {
                        Ok(vec![table])
                    }
                }
                "blockquote" => {
                    let text = serialize_inline_children(&element, remove_inline_citations)?;
                    if text.is_empty() {
                        Ok(Vec::new())
                    } else {
                        Ok(vec![format!("> {}", text)])
                    }
                }
                "figcaption" => {
                    let text = serialize_inline_children(&element, remove_inline_citations)?;
                    if text.is_empty() {
                        Ok(Vec::new())
                    } else {
                        Ok(vec![format!("_{}_", text)])
                    }
                }
                _ => serialize_children(&element, remove_inline_citations),
            }
        }
        _ => Ok(Vec::new()),
    }
}

fn serialize_children(
    element: &ElementRef<'_>,
    remove_inline_citations: bool,
) -> Result<Vec<String>> {
    let mut blocks = Vec::new();
    for child in element.children() {
        blocks.extend(serialize_node(
            child.value(),
            &child,
            remove_inline_citations,
        )?);
    }
    Ok(blocks)
}

fn serialize_inline_children(
    element: &ElementRef<'_>,
    remove_inline_citations: bool,
) -> Result<String> {
    let mut output = String::new();
    for child in element.children() {
        output.push_str(&serialize_inline_node(
            child.value(),
            &child,
            remove_inline_citations,
        )?);
    }
    Ok(clean_inline_text(&output))
}

fn serialize_inline_node(
    node: &Node,
    handle: &NodeRef<'_, Node>,
    remove_inline_citations: bool,
) -> Result<String> {
    match node {
        Node::Text(text) => Ok(text.text.to_string()),
        Node::Element(_) => {
            let Some(element) = ElementRef::wrap(*handle) else {
                return Ok(String::new());
            };
            let name = element.value().name();
            match name {
                "br" => Ok("\n".into()),
                "em" | "i" => Ok(format!(
                    "*{}*",
                    serialize_inline_children(&element, remove_inline_citations)?
                )),
                "strong" | "b" => Ok(format!(
                    "**{}**",
                    serialize_inline_children(&element, remove_inline_citations)?
                )),
                "a" => {
                    let text = serialize_inline_children(&element, remove_inline_citations)?;
                    let href = element.value().attr("href").unwrap_or_default();
                    if is_citation_link(href) {
                        if remove_inline_citations {
                            return Ok(String::new());
                        }
                        return Ok(text);
                    }
                    if is_internal_paper_link(href) {
                        return Ok(text);
                    }
                    if href.is_empty() || text.is_empty() {
                        Ok(text)
                    } else {
                        Ok(format!("[{text}]({href})"))
                    }
                }
                "cite" => {
                    let classes = element.value().classes().collect::<Vec<_>>();
                    if remove_inline_citations
                        && classes
                            .iter()
                            .any(|class_name| class_name.starts_with("ltx_cite"))
                    {
                        return Ok(String::new());
                    }
                    serialize_inline_children(&element, remove_inline_citations)
                }
                "sup" => {
                    let text = serialize_inline_children(&element, remove_inline_citations)?;
                    if text.is_empty() {
                        Ok(String::new())
                    } else {
                        Ok(format!("^{text}"))
                    }
                }
                "math" => Ok(extract_math(&element)),
                _ => serialize_inline_children(&element, remove_inline_citations),
            }
        }
        _ => Ok(String::new()),
    }
}

fn serialize_list(
    element: &ElementRef<'_>,
    indent: usize,
    remove_inline_citations: bool,
) -> Result<Vec<String>> {
    let li_selector = selector(":scope > li");
    let nested_selector = selector("ul, ol");
    let mut lines = Vec::new();
    for item in element.select(&li_selector) {
        let mut item_text = String::new();
        for child in item.children() {
            if let Some(child_element) = ElementRef::wrap(child)
                && nested_selector.matches(&child_element)
            {
                continue;
            }
            item_text.push_str(&serialize_inline_node(
                child.value(),
                &child,
                remove_inline_citations,
            )?);
        }
        lines.push(format!(
            "{}- {}",
            "  ".repeat(indent),
            clean_inline_text(&item_text)
        ));
        for nested in item.select(&nested_selector) {
            lines.extend(serialize_list(
                &nested,
                indent + 1,
                remove_inline_citations,
            )?);
        }
    }
    Ok(lines)
}

fn serialize_table(element: &ElementRef<'_>, remove_inline_citations: bool) -> Result<String> {
    let classes = element.value().classes().collect::<Vec<_>>();
    if classes.iter().any(|class_name| {
        matches!(
            *class_name,
            "ltx_equationgroup" | "ltx_eqn_align" | "ltx_eqn_table"
        )
    }) {
        let text = normalize_text(&element.text().collect::<Vec<_>>().join(" "));
        return Ok(if text.is_empty() {
            String::new()
        } else {
            format!("$$ {} $$", text)
        });
    }

    let row_selector = selector("tr");
    let cell_selector = selector("th, td");
    let mut rows = Vec::new();
    for row in element.select(&row_selector) {
        let values = row
            .select(&cell_selector)
            .map(|cell| serialize_inline_children(&cell, remove_inline_citations))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(|value| value.replace('\n', "<br>"))
            .collect::<Vec<_>>();
        if !values.is_empty() {
            rows.push(values);
        }
    }
    if rows.is_empty() {
        return Ok(String::new());
    }

    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    if column_count == 0 {
        return Ok(String::new());
    }

    for row in &mut rows {
        while row.len() < column_count {
            row.push(String::new());
        }
    }

    let header = rows.remove(0);
    let mut lines = Vec::new();
    lines.push(format!("| {} |", header.join(" | ")));
    lines.push(format!("| {} |", vec!["---"; column_count].join(" | ")));
    for row in rows {
        lines.push(format!("| {} |", row.join(" | ")));
    }
    Ok(lines.join("\n"))
}

fn extract_math(element: &ElementRef<'_>) -> String {
    let annotation_selector = selector("annotation");
    if let Some(annotation) = element
        .select(&annotation_selector)
        .find(|annotation| annotation.value().attr("encoding") == Some("application/x-tex"))
    {
        let latex = clean_inline_text(&annotation.text().collect::<Vec<_>>().join(" "));
        if !latex.is_empty() {
            return format!("${latex}$");
        }
    }
    let fallback = clean_inline_text(&element.text().collect::<Vec<_>>().join(" "));
    if fallback.is_empty() {
        String::new()
    } else {
        format!("${fallback}$")
    }
}

fn build_tree(flat: Vec<(u8, String, Option<String>, String)>) -> Vec<SectionNode> {
    #[derive(Clone)]
    struct TempNode {
        title: String,
        level: u8,
        anchor: Option<String>,
        markdown: String,
        children: Vec<usize>,
    }

    let mut nodes = Vec::<TempNode>::new();
    let mut roots = Vec::<usize>::new();
    let mut stack = Vec::<usize>::new();

    for (level, title, anchor, markdown) in flat {
        let index = nodes.len();
        nodes.push(TempNode {
            title,
            level,
            anchor,
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
            anchor: node.anchor.clone(),
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

fn select_first<'a>(document: &'a Html, css: &str) -> Option<ElementRef<'a>> {
    document.select(&selector(css)).next()
}

fn select_first_text(document: &Html, css: &str) -> Option<String> {
    select_first(document, css)
        .map(|element| normalize_text(&element.text().collect::<Vec<_>>().join(" ")))
}

fn selector(css: &str) -> Selector {
    Selector::parse(css).expect("valid selector")
}

fn collect_section_fragment(heading: &ElementRef<'_>) -> String {
    let heading_html = heading.html();
    let mut parent_section = None;
    for ancestor in heading.ancestors() {
        if let Some(element) = ElementRef::wrap(ancestor)
            && element.value().name() == "section"
        {
            parent_section = Some(element);
            break;
        }
    }
    let Some(section) = parent_section else {
        return String::new();
    };

    let mut started = false;
    let mut parts = Vec::new();
    for child in section.children() {
        if let Some(element) = ElementRef::wrap(child) {
            if !started {
                if element.html() == heading_html {
                    started = true;
                }
                continue;
            }
            if element.value().name() == "section" {
                continue;
            }
            parts.push(element.html());
        } else if started && let Node::Text(text) = child.value() {
            parts.push(text.text.to_string());
        }
    }
    parts.join("")
}

fn has_ancestor(element: &ElementRef<'_>, tag_name: &str) -> bool {
    element.ancestors().any(|ancestor| {
        ElementRef::wrap(ancestor)
            .map(|value| value.value().name() == tag_name)
            .unwrap_or(false)
    })
}

fn has_ancestor_class(element: &ElementRef<'_>, class_name: &str) -> bool {
    element.ancestors().any(|ancestor| {
        ElementRef::wrap(ancestor)
            .map(|value| {
                value
                    .value()
                    .classes()
                    .any(|candidate| candidate == class_name)
            })
            .unwrap_or(false)
    })
}

fn is_citation_link(href: &str) -> bool {
    href.contains("#bib.") || href.starts_with("#bib")
}

fn is_internal_paper_link(href: &str) -> bool {
    href.contains("arxiv.org/html/") && href.contains('#')
}

fn clean_author_text(raw: &str) -> Vec<String> {
    let email_re = Regex::new(r"^[\w.+-]+@[\w.-]+\.\w+$").expect("valid regex");
    let mut cleaned = Vec::new();
    for part in raw.lines().flat_map(|line| line.split('\n')) {
        let value = normalize_text(part.trim().trim_start_matches('&'));
        if value.is_empty()
            || value.chars().all(|ch| ch.is_ascii_digit())
            || email_re.is_match(&value)
        {
            continue;
        }
        let lower = value.to_lowercase();
        if lower.contains("equal contribution")
            || lower.contains("footnotemark:")
            || lower.contains("work performed")
            || lower.contains("listing order")
        {
            continue;
        }
        if value.len() > 80 {
            continue;
        }
        cleaned.push(value);
    }
    cleaned
}

fn normalize_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clean_inline_text(value: &str) -> String {
    let newline_re = Regex::new(r"\s*\n\s*").expect("valid regex");
    let space_re = Regex::new(r"[ \t]+").expect("valid regex");
    let collapsed = newline_re.replace_all(value, "\n");
    let collapsed = space_re.replace_all(&collapsed, " ");
    collapsed.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::parse_document;

    #[test]
    fn parses_html_sections_and_math() {
        let html = r#"
<html>
  <body>
    <article class="ltx_document">
      <h1 class="ltx_title ltx_title_document">Sample Title</h1>
      <div class="ltx_authors">
        <span class="ltx_text ltx_font_bold">Alice<sup>1</sup></span>
        <span class="ltx_text ltx_font_bold">Bob<sup>2</sup></span>
      </div>
      <div class="ltx_abstract"><p>Abstract text.</p></div>
      <section class="ltx_section" id="S1">
        <h2 class="ltx_title ltx_title_section">1 Intro</h2>
        <p>Equation <math><annotation encoding="application/x-tex">x+y</annotation></math></p>
      </section>
      <section class="ltx_section" id="S2">
        <h2 class="ltx_title ltx_title_section">2 Results</h2>
        <figure>
          <table class="ltx_tabular">
            <tr><th>A</th><th>B</th></tr>
            <tr><td>1</td><td>2</td></tr>
          </table>
          <figcaption>Table caption.</figcaption>
        </figure>
      </section>
    </article>
  </body>
</html>
"#;

        let parsed = parse_document(html, false).expect("html parses");
        assert_eq!(parsed.title.as_deref(), Some("Sample Title"));
        assert_eq!(parsed.authors, vec!["Alice 1", "Bob 2"]);
        assert_eq!(parsed.abstract_text.as_deref(), Some("Abstract text."));
        assert_eq!(parsed.sections.len(), 2);
        assert!(parsed.sections[0].markdown.contains("$x+y$"));
        assert!(parsed.sections[1].markdown.contains("| A | B |"));
        assert!(parsed.sections[1].markdown.contains("_Table caption._"));
    }
}
