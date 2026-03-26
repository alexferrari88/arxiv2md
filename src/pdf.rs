use crate::error::{Arxiv2MdError, Result};

pub fn extract_text(bytes: &[u8]) -> Result<String> {
    let text = pdf_extract::extract_text_from_mem(bytes)
        .map_err(|error| Arxiv2MdError::Pdf(error.to_string()))?;
    Ok(normalize_pdf_text(&text))
}

fn normalize_pdf_text(text: &str) -> String {
    let normalized = text
        .replace('\r', "")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    crate::markdown::collapse_blank_lines(&normalized)
}
