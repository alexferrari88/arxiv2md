use crate::error::{Arxiv2MdError, Result};
use crate::model::PaperMetadata;

pub fn parse_atom(xml: &str) -> Result<PaperMetadata> {
    let document = roxmltree::Document::parse(xml)
        .map_err(|error| Arxiv2MdError::Metadata(error.to_string()))?;
    let entry = document
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "entry")
        .ok_or_else(|| Arxiv2MdError::Metadata("atom feed missing entry".into()))?;

    let mut metadata = PaperMetadata::default();
    metadata.title = child_text(entry, "title");
    metadata.abstract_text = child_text(entry, "summary");
    metadata.published = child_text(entry, "published");
    metadata.updated = child_text(entry, "updated");

    metadata.authors = entry
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "author")
        .filter_map(|author| child_text(author, "name"))
        .collect();

    metadata.categories = entry
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "category")
        .filter_map(|category| category.attribute("term").map(ToOwned::to_owned))
        .collect();

    Ok(metadata)
}

fn child_text(node: roxmltree::Node<'_, '_>, name: &str) -> Option<String> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
        .and_then(|child| child.text())
        .map(normalize_text)
}

fn normalize_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::parse_atom;

    #[test]
    fn parses_atom_metadata() {
        let xml = r#"<?xml version='1.0' encoding='UTF-8'?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <title> Sample Title </title>
    <summary> Sample abstract. </summary>
    <published>2025-01-19T17:28:12Z</published>
    <updated>2025-01-20T17:28:12Z</updated>
    <author><name>Alice</name></author>
    <author><name>Bob</name></author>
    <category term="cs.CL" />
    <category term="cs.AI" />
  </entry>
</feed>"#;

        let metadata = parse_atom(xml).expect("metadata parses");
        assert_eq!(metadata.title.as_deref(), Some("Sample Title"));
        assert_eq!(metadata.abstract_text.as_deref(), Some("Sample abstract."));
        assert_eq!(metadata.authors, vec!["Alice", "Bob"]);
        assert_eq!(metadata.categories, vec!["cs.CL", "cs.AI"]);
    }
}
