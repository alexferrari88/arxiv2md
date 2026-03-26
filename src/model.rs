use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionFilterMode {
    Include,
    Exclude,
}

#[derive(Debug, Clone)]
pub struct CliOptions {
    pub input: String,
    pub output: String,
    pub frontmatter: bool,
    pub keep_refs: bool,
    pub keep_toc: bool,
    pub remove_inline_citations: bool,
    pub include_tree: bool,
    pub refresh: bool,
    pub sections: Vec<String>,
    pub section_filter_mode: SectionFilterMode,
    pub cache_dir: Option<PathBuf>,
    pub pandoc_path: String,
}

#[derive(Debug, Clone)]
pub struct ArxivId {
    pub normalized: String,
    pub version: Option<String>,
    pub versioned_input: bool,
    pub abs_url: String,
    pub html_url: String,
    pub ar5iv_url: String,
    pub pdf_url: String,
    pub eprint_url: String,
}

#[derive(Debug, Clone, Default)]
pub struct PaperMetadata {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub abstract_text: Option<String>,
    pub categories: Vec<String>,
    pub published: Option<String>,
    pub updated: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SectionNode {
    pub title: String,
    pub level: u8,
    pub anchor: Option<String>,
    pub markdown: String,
    pub children: Vec<SectionNode>,
}

#[derive(Debug, Clone)]
pub struct HtmlPaper {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub abstract_text: Option<String>,
    pub sections: Vec<SectionNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedVia {
    ArxivHtml,
    Ar5iv,
    LatexSource,
    Pdf,
}

#[derive(Debug, Clone)]
pub struct PaperDocument {
    pub metadata: PaperMetadata,
    pub sections: Vec<SectionNode>,
    pub fallback_markdown: Option<String>,
    pub resolved_via: ResolvedVia,
    pub supports_tree: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub arxiv_id: String,
    pub version: Option<String>,
    pub authors: Vec<String>,
    pub categories: Vec<String>,
    pub published: Option<String>,
    pub updated: Option<String>,
    pub abs_url: String,
    pub resolved_via: ResolvedVia,
}

#[derive(Debug, Clone)]
pub struct ConversionOutput {
    pub markdown: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CachePolicy {
    pub dir: PathBuf,
    pub ttl: Option<Duration>,
    pub refresh: bool,
}

#[derive(Debug, Clone)]
pub struct FetchedText {
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct FetchedBytes {
    pub body: Vec<u8>,
    pub content_type: Option<String>,
}
