use std::fmt::Write;

use crate::db::FeedItem;

/// Generate an Atom 1.0 XML feed from a list of feed items.
pub fn generate_atom(items: &[FeedItem]) -> String {
    let updated = items
        .first()
        .map_or("1970-01-01T00:00:00Z", |i| i.created_at.as_str());

    let mut xml = String::with_capacity(4096);
    xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str("<feed xmlns=\"http://www.w3.org/2005/Atom\">\n");
    xml.push_str("  <title>myfeed</title>\n");
    xml.push_str("  <id>urn:myfeed</id>\n");
    let _ = writeln!(xml, "  <updated>{updated}</updated>");

    for item in items {
        let title = escape_xml(&format!("[{}] {}", item.site, item.title));
        let url = escape_xml(&item.url);
        let id = escape_xml(&format!("{}:{}", item.site, item.external_id));
        let content = escape_xml(&item.preview);
        let published = escape_xml(&item.created_at);
        let site = escape_xml(&item.site);

        xml.push_str("  <entry>\n");
        let _ = writeln!(xml, "    <title>{title}</title>");
        let _ = writeln!(xml, "    <link href=\"{url}\" />");
        let _ = writeln!(xml, "    <id>{id}</id>");
        let _ = writeln!(xml, "    <published>{published}</published>");
        let _ = writeln!(xml, "    <updated>{published}</updated>");
        let _ = writeln!(xml, "    <content type=\"text\">{content}</content>");
        let _ = writeln!(xml, "    <category term=\"{site}\" />");
        xml.push_str("  </entry>\n");
    }

    xml.push_str("</feed>\n");
    xml
}

fn escape_xml(s: &str) -> String {
    crate::telegram::escape_html(s).replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_feed() {
        let xml = generate_atom(&[]);
        assert!(xml.contains("<feed"));
        assert!(xml.contains("</feed>"));
        assert!(xml.contains("<title>myfeed</title>"));
    }

    #[test]
    fn escapes_xml_special_chars() {
        let items = vec![FeedItem {
            id: 1,
            site: "test".to_string(),
            external_id: "1".to_string(),
            title: "Title with <html> & \"quotes\"".to_string(),
            url: "https://example.com?a=1&b=2".to_string(),
            preview: "Content with <tags>".to_string(),
            raw_json: "{}".to_string(),
            created_at: "2026-03-23T00:00:00Z".to_string(),
        }];
        let xml = generate_atom(&items);
        assert!(xml.contains("&lt;html&gt;"));
        assert!(xml.contains("&amp;"));
        assert!(xml.contains("&quot;quotes&quot;"));
        assert!(!xml.contains("<html>"));
    }
}
