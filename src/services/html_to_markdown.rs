/// Simple HTML-to-Markdown converter for importing content from WordPress/Ghost.
/// Handles common HTML elements without requiring a full DOM parser.

pub fn convert(html: &str) -> String {
    let mut result = html.to_string();

    // Remove CDATA sections
    result = result.replace("<![CDATA[", "").replace("]]>", "");

    // Handle code blocks first (before stripping other tags)
    result = convert_code_blocks(&result);

    // Convert block-level elements
    result = convert_headings(&result);
    result = convert_paragraphs(&result);
    result = convert_blockquotes(&result);
    result = convert_lists(&result);
    result = convert_horizontal_rules(&result);

    // Convert inline elements
    result = convert_links(&result);
    result = convert_images(&result);
    result = convert_bold(&result);
    result = convert_italic(&result);
    result = convert_inline_code(&result);

    // Convert line breaks
    result = result.replace("<br>", "\n");
    result = result.replace("<br/>", "\n");
    result = result.replace("<br />", "\n");

    // Remove remaining HTML tags
    result = strip_tags(&result);

    // Decode common HTML entities
    result = decode_entities(&result);

    // Clean up excessive whitespace
    clean_whitespace(&result)
}

fn convert_headings(html: &str) -> String {
    let mut result = html.to_string();
    for level in 1..=6 {
        let prefix = "#".repeat(level);
        let open = format!("<h{}", level);
        let close = format!("</h{}>", level);

        while let Some(start) = result.find(&open) {
            if let Some(gt) = result[start..].find('>') {
                let tag_end = start + gt + 1;
                if let Some(end) = result[tag_end..].find(&close) {
                    let content = result[tag_end..tag_end + end].trim();
                    let replacement = format!("\n\n{} {}\n\n", prefix, content);
                    result = format!("{}{}{}", &result[..start], replacement, &result[tag_end + end + close.len()..]);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    result
}

fn convert_paragraphs(html: &str) -> String {
    let mut result = html.to_string();
    // Handle <p> with attributes
    while let Some(start) = result.find("<p") {
        if let Some(gt) = result[start..].find('>') {
            let tag_end = start + gt + 1;
            if let Some(end) = result[tag_end..].find("</p>") {
                let content = result[tag_end..tag_end + end].trim();
                let replacement = format!("\n\n{}\n\n", content);
                result = format!("{}{}{}", &result[..start], replacement, &result[tag_end + end + 4..]);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    result
}

fn convert_blockquotes(html: &str) -> String {
    let mut result = html.to_string();
    while let Some(start) = result.find("<blockquote") {
        if let Some(gt) = result[start..].find('>') {
            let tag_end = start + gt + 1;
            if let Some(end) = result[tag_end..].find("</blockquote>") {
                let content = result[tag_end..tag_end + end].trim();
                let quoted = content.lines().map(|l| format!("> {}", l.trim())).collect::<Vec<_>>().join("\n");
                let replacement = format!("\n\n{}\n\n", quoted);
                result = format!("{}{}{}", &result[..start], replacement, &result[tag_end + end + 13..]);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    result
}

fn convert_lists(html: &str) -> String {
    let mut result = html.to_string();

    // Convert <li> items first
    while let Some(start) = result.find("<li") {
        if let Some(gt) = result[start..].find('>') {
            let tag_end = start + gt + 1;
            if let Some(end) = result[tag_end..].find("</li>") {
                let content = result[tag_end..tag_end + end].trim();
                let replacement = format!("\n- {}", content);
                result = format!("{}{}{}", &result[..start], replacement, &result[tag_end + end + 5..]);
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Strip list wrappers
    for tag in &["ul", "ol"] {
        let open = format!("<{}", tag);
        let close = format!("</{}>", tag);
        while let Some(start) = result.find(&open) {
            if let Some(gt) = result[start..].find('>') {
                let tag_end = start + gt + 1;
                if let Some(end) = result[tag_end..].find(&close) {
                    let content = &result[tag_end..tag_end + end];
                    let replacement = format!("\n{}\n", content);
                    result = format!("{}{}{}", &result[..start], replacement, &result[tag_end + end + close.len()..]);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    result
}

fn convert_code_blocks(html: &str) -> String {
    let mut result = html.to_string();
    // <pre><code>...</code></pre>
    while let Some(start) = result.find("<pre") {
        if let Some(gt) = result[start..].find('>') {
            let tag_end = start + gt + 1;
            if let Some(end) = result[tag_end..].find("</pre>") {
                let mut content = result[tag_end..tag_end + end].to_string();
                // Strip inner <code> tags
                if content.starts_with("<code") {
                    if let Some(code_gt) = content.find('>') {
                        content = content[code_gt + 1..].to_string();
                    }
                }
                if content.ends_with("</code>") {
                    content = content[..content.len() - 7].to_string();
                }
                let replacement = format!("\n\n```\n{}\n```\n\n", content.trim());
                result = format!("{}{}{}", &result[..start], replacement, &result[tag_end + end + 6..]);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    result
}

fn convert_horizontal_rules(html: &str) -> String {
    let mut result = html.to_string();
    result = result.replace("<hr>", "\n\n---\n\n");
    result = result.replace("<hr/>", "\n\n---\n\n");
    result = result.replace("<hr />", "\n\n---\n\n");
    result
}

fn convert_links(html: &str) -> String {
    let mut result = html.to_string();
    while let Some(start) = result.find("<a ") {
        if let Some(gt) = result[start..].find('>') {
            let tag_content = &result[start..start + gt];
            let href = extract_attr(tag_content, "href");
            let tag_end = start + gt + 1;
            if let Some(end) = result[tag_end..].find("</a>") {
                let text = result[tag_end..tag_end + end].trim();
                let replacement = if let Some(ref url) = href {
                    format!("[{}]({})", text, url)
                } else {
                    text.to_string()
                };
                result = format!("{}{}{}", &result[..start], replacement, &result[tag_end + end + 4..]);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    result
}

fn convert_images(html: &str) -> String {
    let mut result = html.to_string();
    while let Some(start) = result.find("<img ") {
        // Find the end of the tag (could be /> or >)
        let search = &result[start..];
        let end = search.find("/>").map(|p| p + 2)
            .or_else(|| search.find('>').map(|p| p + 1));
        if let Some(end_offset) = end {
            let tag_content = &result[start..start + end_offset];
            let src = extract_attr(tag_content, "src").unwrap_or_default();
            let alt = extract_attr(tag_content, "alt").unwrap_or_default();
            let replacement = format!("![{}]({})", alt, src);
            result = format!("{}{}{}", &result[..start], replacement, &result[start + end_offset..]);
        } else {
            break;
        }
    }
    result
}

fn convert_bold(html: &str) -> String {
    let mut result = html.to_string();
    for tag in &["strong", "b"] {
        let open = format!("<{}>", tag);
        let close = format!("</{}>", tag);
        while let Some(start) = result.find(&open) {
            let tag_end = start + open.len();
            if let Some(end) = result[tag_end..].find(&close) {
                let content = &result[tag_end..tag_end + end];
                let replacement = format!("**{}**", content);
                result = format!("{}{}{}", &result[..start], replacement, &result[tag_end + end + close.len()..]);
            } else {
                break;
            }
        }
    }
    result
}

fn convert_italic(html: &str) -> String {
    let mut result = html.to_string();
    for tag in &["em", "i"] {
        let open = format!("<{}>", tag);
        let close = format!("</{}>", tag);
        while let Some(start) = result.find(&open) {
            let tag_end = start + open.len();
            if let Some(end) = result[tag_end..].find(&close) {
                let content = &result[tag_end..tag_end + end];
                let replacement = format!("*{}*", content);
                result = format!("{}{}{}", &result[..start], replacement, &result[tag_end + end + close.len()..]);
            } else {
                break;
            }
        }
    }
    result
}

fn convert_inline_code(html: &str) -> String {
    let mut result = html.to_string();
    while let Some(start) = result.find("<code>") {
        let tag_end = start + 6;
        if let Some(end) = result[tag_end..].find("</code>") {
            let content = &result[tag_end..tag_end + end];
            let replacement = format!("`{}`", content);
            result = format!("{}{}{}", &result[..start], replacement, &result[tag_end + end + 7..]);
        } else {
            break;
        }
    }
    result
}

fn extract_attr(tag: &str, attr_name: &str) -> Option<String> {
    let search = format!("{}=\"", attr_name);
    if let Some(start) = tag.find(&search) {
        let value_start = start + search.len();
        if let Some(end) = tag[value_start..].find('"') {
            return Some(tag[value_start..value_start + end].to_string());
        }
    }
    // Try single quotes
    let search = format!("{}='", attr_name);
    if let Some(start) = tag.find(&search) {
        let value_start = start + search.len();
        if let Some(end) = tag[value_start..].find('\'') {
            return Some(tag[value_start..value_start + end].to_string());
        }
    }
    None
}

fn strip_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            if in_tag {
                in_tag = false;
            } else {
                // Standalone '>' not part of an HTML tag (e.g. markdown blockquote)
                result.push(ch);
            }
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

fn decode_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&#8220;", "\u{201c}")
        .replace("&#8221;", "\u{201d}")
        .replace("&#8216;", "\u{2018}")
        .replace("&#8217;", "\u{2019}")
        .replace("&#8212;", "\u{2014}")
        .replace("&#8211;", "\u{2013}")
}

fn clean_whitespace(text: &str) -> String {
    let mut result = text.to_string();
    // Collapse 3+ newlines to 2
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }
    // Trim trailing whitespace from each line
    result = result
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_basic_html() {
        let html = "<p>Hello <strong>world</strong></p>";
        let md = convert(html);
        assert!(md.contains("Hello **world**"));
    }

    #[test]
    fn test_convert_headings() {
        let html = "<h1>Title</h1><h2>Subtitle</h2>";
        let md = convert(html);
        assert!(md.contains("# Title"));
        assert!(md.contains("## Subtitle"));
    }

    #[test]
    fn test_convert_links() {
        let html = r#"<a href="https://example.com">Click here</a>"#;
        let md = convert(html);
        assert!(md.contains("[Click here](https://example.com)"));
    }

    #[test]
    fn test_convert_images() {
        let html = r#"<img src="/photo.jpg" alt="A photo" />"#;
        let md = convert(html);
        assert!(md.contains("![A photo](/photo.jpg)"));
    }

    #[test]
    fn test_convert_code_block() {
        let html = "<pre><code>fn main() {}</code></pre>";
        let md = convert(html);
        assert!(md.contains("```\nfn main() {}\n```"));
    }

    #[test]
    fn test_convert_blockquote() {
        let html = "<blockquote>A wise quote</blockquote>";
        let md = convert(html);
        assert!(md.contains("> A wise quote"));
    }

    #[test]
    fn test_convert_list() {
        let html = "<ul><li>One</li><li>Two</li></ul>";
        let md = convert(html);
        assert!(md.contains("- One"));
        assert!(md.contains("- Two"));
    }

    #[test]
    fn test_entities() {
        let html = "<p>Tom &amp; Jerry &lt;3</p>";
        let md = convert(html);
        assert!(md.contains("Tom & Jerry <3"));
    }
}
