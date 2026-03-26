use regex::Regex;

use crate::error::{Arxiv2MdError, Result};
use crate::model::ArxivId;

pub fn parse(input: &str) -> Result<ArxivId> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err(Arxiv2MdError::InvalidInput(input.to_owned()));
    }

    let stripped = raw
        .strip_prefix("arXiv:")
        .or_else(|| raw.strip_prefix("arxiv:"))
        .unwrap_or(raw);
    let (normalized, version, versioned_input) = if looks_like_url(stripped) {
        extract_from_url(stripped)?
    } else {
        let (id, version) = normalize_id(stripped)?;
        let versioned = version.is_some();
        (id, version, versioned)
    };

    Ok(ArxivId {
        normalized: normalized.clone(),
        version,
        versioned_input,
        abs_url: format!("https://arxiv.org/abs/{normalized}"),
        html_url: format!("https://arxiv.org/html/{normalized}"),
        ar5iv_url: format!("https://ar5iv.labs.arxiv.org/html/{normalized}"),
        pdf_url: format!("https://arxiv.org/pdf/{normalized}.pdf"),
        eprint_url: format!("https://arxiv.org/e-print/{normalized}"),
    })
}

fn looks_like_url(value: &str) -> bool {
    value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("arxiv.org/")
        || matches!(value.split('/').next(), Some("abs" | "html" | "pdf"))
}

fn extract_from_url(raw: &str) -> Result<(String, Option<String>, bool)> {
    let mut value = raw.to_owned();
    if value.starts_with("arxiv.org/") {
        value = format!("https://{value}");
    }
    if !value.starts_with("http://") && !value.starts_with("https://") {
        value = format!("https://arxiv.org/{value}");
    }

    let parsed =
        reqwest::Url::parse(&value).map_err(|_| Arxiv2MdError::InvalidInput(raw.to_owned()))?;
    let host = parsed.host_str().unwrap_or_default();
    if !host.ends_with("arxiv.org") && !host.ends_with("ar5iv.labs.arxiv.org") {
        return Err(Arxiv2MdError::UnsupportedHost(host.to_owned()));
    }

    let mut parts = parsed
        .path_segments()
        .map(|segments| {
            segments
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if parts.is_empty() {
        return Err(Arxiv2MdError::InvalidInput(raw.to_owned()));
    }

    let candidate = if matches!(parts.first(), Some(&"abs" | &"html" | &"pdf")) {
        if parts.len() < 2 {
            return Err(Arxiv2MdError::InvalidInput(raw.to_owned()));
        }
        let mut id = parts.remove(1).to_owned();
        if id.ends_with(".pdf") {
            id.truncate(id.len() - 4);
        }
        id
    } else {
        parts.remove(0).to_owned()
    };

    let (normalized, version) = normalize_id(&candidate)?;
    Ok((normalized, version.clone(), version.is_some()))
}

fn normalize_id(value: &str) -> Result<(String, Option<String>)> {
    let re = Regex::new(r"^(?P<base>(\d{4}\.\d{4,5}|[A-Za-z-]+/\d{7}))(v(?P<version>\d+))?$")
        .expect("valid regex");
    let captures = re
        .captures(value.trim())
        .ok_or_else(|| Arxiv2MdError::InvalidInput(value.to_owned()))?;
    let base = captures
        .name("base")
        .map(|item| item.as_str())
        .unwrap_or_default();
    let version = captures
        .name("version")
        .map(|item| format!("v{}", item.as_str()));
    let normalized = if let Some(version) = &version {
        format!("{base}{version}")
    } else {
        base.to_owned()
    };
    Ok((normalized, version))
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_bare_id() {
        let parsed = parse("2501.11120v1").expect("valid id");
        assert_eq!(parsed.normalized, "2501.11120v1");
        assert_eq!(parsed.version.as_deref(), Some("v1"));
    }

    #[test]
    fn parses_old_style_id() {
        let parsed = parse("hep-th/9901001").expect("valid id");
        assert_eq!(parsed.normalized, "hep-th/9901001");
        assert!(parsed.version.is_none());
    }

    #[test]
    fn parses_pdf_url() {
        let parsed = parse("https://arxiv.org/pdf/2501.11120v2.pdf").expect("valid id");
        assert_eq!(parsed.normalized, "2501.11120v2");
        assert_eq!(parsed.version.as_deref(), Some("v2"));
    }
}
