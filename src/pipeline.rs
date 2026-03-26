use std::fs;
use std::path::PathBuf;

use reqwest::StatusCode;

use crate::cache;
use crate::error::{Arxiv2MdError, Result};
use crate::model::{
    ArxivId, CliOptions, ConversionOutput, FetchedBytes, FetchedText, HtmlPaper, PaperDocument,
    PaperMetadata, ResolvedVia,
};

pub fn run() -> Result<()> {
    let options = crate::cli::parse();
    let id = crate::id::parse(&options.input)?;
    let cache_policy = cache::build_policy(&id, options.cache_dir.clone(), options.refresh)?;
    let client = http_client()?;

    let mut warnings = Vec::new();
    let metadata = fetch_metadata(&client, &id, &cache_policy)?;
    let document = resolve_document(
        &client,
        &id,
        &options,
        &cache_policy,
        metadata,
        &mut warnings,
    )?;
    let output = crate::markdown::render(&id, &options, document, warnings)?;
    write_output(&options, output)
}

fn resolve_document(
    client: &reqwest::blocking::Client,
    id: &ArxivId,
    options: &CliOptions,
    cache_policy: &crate::model::CachePolicy,
    metadata: PaperMetadata,
    warnings: &mut Vec<String>,
) -> Result<PaperDocument> {
    if let Some(primary) = fetch_text_maybe(client, cache_policy, "primary.html", &id.html_url)? {
        if let Ok(parsed) =
            crate::html::parse_document(&primary.body, options.remove_inline_citations)
        {
            return Ok(html_document(metadata, parsed, ResolvedVia::ArxivHtml));
        }
        warnings.push(format!(
            "ignoring non-arXiv HTML response from {}",
            id.html_url
        ));
    }

    if let Some(ar5iv) = fetch_text_maybe(client, cache_policy, "ar5iv.html", &id.ar5iv_url)? {
        if let Ok(parsed) =
            crate::html::parse_document(&ar5iv.body, options.remove_inline_citations)
        {
            return Ok(html_document(metadata, parsed, ResolvedVia::Ar5iv));
        }
        warnings.push(format!(
            "ignoring non-arXiv HTML response from {}",
            id.ar5iv_url
        ));
    }

    if let Some(source) = fetch_bytes_maybe(client, cache_policy, "source.bin", &id.eprint_url)?
        && !looks_like_pdf(&source.body, source.content_type.as_deref())
    {
        match crate::latex::convert_source_archive(&source.body, &options.pandoc_path) {
            Ok(markdown) => {
                let cleaned = crate::markdown::cleanup_generated_markdown(&markdown);
                let sections = crate::markdown::parse_markdown_sections(&cleaned);
                let fallback_markdown = if sections.is_empty() {
                    Some(cleaned)
                } else {
                    None
                };
                return Ok(PaperDocument {
                    metadata,
                    sections,
                    fallback_markdown,
                    resolved_via: ResolvedVia::LatexSource,
                    supports_tree: true,
                });
            }
            Err(error) => warnings.push(format!(
                "source fallback failed for {} using {}: {}",
                id.normalized, options.pandoc_path, error
            )),
        }
    }

    let pdf = fetch_bytes_required(client, cache_policy, "paper.pdf", &id.pdf_url)?;
    let mut text = crate::pdf::extract_text(&pdf.body)?;
    if !options.keep_refs {
        text = crate::markdown::remove_references_from_text(&text);
    }
    if options.remove_inline_citations {
        warnings.push("inline citation removal is not supported for PDF fallback; returning raw extracted text".into());
    }
    if options.include_tree || !options.sections.is_empty() {
        warnings
            .push("section tree and section filtering are not available for PDF fallback".into());
    }
    Ok(PaperDocument {
        metadata,
        sections: Vec::new(),
        fallback_markdown: Some(text),
        resolved_via: ResolvedVia::Pdf,
        supports_tree: false,
    })
}

fn html_document(
    metadata: PaperMetadata,
    parsed: HtmlPaper,
    resolved_via: ResolvedVia,
) -> PaperDocument {
    let mut merged = metadata;
    if merged.title.is_none() {
        merged.title = parsed.title;
    }
    if merged.authors.is_empty() {
        merged.authors = parsed.authors;
    }
    if merged.abstract_text.is_none() {
        merged.abstract_text = parsed.abstract_text;
    }
    PaperDocument {
        metadata: merged,
        sections: parsed.sections,
        fallback_markdown: None,
        resolved_via,
        supports_tree: true,
    }
}

fn fetch_metadata(
    client: &reqwest::blocking::Client,
    id: &ArxivId,
    cache_policy: &crate::model::CachePolicy,
) -> Result<PaperMetadata> {
    let url = format!(
        "https://export.arxiv.org/api/query?id_list={}",
        id.normalized
    );
    let response = fetch_text_required(client, cache_policy, "metadata.xml", &url)?;
    crate::metadata::parse_atom(&response.body)
}

fn fetch_text_required(
    client: &reqwest::blocking::Client,
    cache_policy: &crate::model::CachePolicy,
    cache_name: &str,
    url: &str,
) -> Result<FetchedText> {
    fetch_text(client, cache_policy, cache_name, url)?
        .ok_or_else(|| Arxiv2MdError::NotFound(url.to_owned()))
}

fn fetch_bytes_required(
    client: &reqwest::blocking::Client,
    cache_policy: &crate::model::CachePolicy,
    cache_name: &str,
    url: &str,
) -> Result<FetchedBytes> {
    fetch_bytes(client, cache_policy, cache_name, url)?
        .ok_or_else(|| Arxiv2MdError::NotFound(url.to_owned()))
}

fn fetch_text_maybe(
    client: &reqwest::blocking::Client,
    cache_policy: &crate::model::CachePolicy,
    cache_name: &str,
    url: &str,
) -> Result<Option<FetchedText>> {
    fetch_text(client, cache_policy, cache_name, url)
}

fn fetch_bytes_maybe(
    client: &reqwest::blocking::Client,
    cache_policy: &crate::model::CachePolicy,
    cache_name: &str,
    url: &str,
) -> Result<Option<FetchedBytes>> {
    fetch_bytes(client, cache_policy, cache_name, url)
}

fn fetch_text(
    client: &reqwest::blocking::Client,
    cache_policy: &crate::model::CachePolicy,
    cache_name: &str,
    url: &str,
) -> Result<Option<FetchedText>> {
    if let Some(body) = cache::read_text(cache_policy, cache_name)? {
        return Ok(Some(FetchedText { body }));
    }
    let response = client
        .get(url)
        .send()
        .map_err(|error| Arxiv2MdError::Network(error.to_string()))?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Ok(None);
    }
    let body = response
        .text()
        .map_err(|error| Arxiv2MdError::Network(error.to_string()))?;
    cache::write_text(cache_policy, cache_name, &body)?;
    Ok(Some(FetchedText { body }))
}

fn fetch_bytes(
    client: &reqwest::blocking::Client,
    cache_policy: &crate::model::CachePolicy,
    cache_name: &str,
    url: &str,
) -> Result<Option<FetchedBytes>> {
    if let Some(body) = cache::read_bytes(cache_policy, cache_name)? {
        return Ok(Some(FetchedBytes {
            body,
            content_type: None,
        }));
    }
    let response = client
        .get(url)
        .send()
        .map_err(|error| Arxiv2MdError::Network(error.to_string()))?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Ok(None);
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let body = response
        .bytes()
        .map_err(|error| Arxiv2MdError::Network(error.to_string()))?
        .to_vec();
    cache::write_bytes(cache_policy, cache_name, &body)?;
    Ok(Some(FetchedBytes { body, content_type }))
}

fn http_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent("arxiv2md/0.1 (+https://arxiv.org)")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|error| Arxiv2MdError::Network(error.to_string()))
}

fn looks_like_pdf(bytes: &[u8], content_type: Option<&str>) -> bool {
    content_type
        .map(|value| value.to_ascii_lowercase().contains("application/pdf"))
        .unwrap_or(false)
        || bytes.starts_with(b"%PDF-")
}

fn write_output(options: &CliOptions, output: ConversionOutput) -> Result<()> {
    for warning in &output.warnings {
        eprintln!("Warning: {warning}");
    }
    if options.output == "-" {
        print!("{}", output.markdown);
        if !output.markdown.ends_with('\n') {
            println!();
        }
        return Ok(());
    }
    let path = PathBuf::from(&options.output);
    fs::write(&path, output.markdown)
        .map_err(|error| Arxiv2MdError::Output(path, error.to_string()))
}
