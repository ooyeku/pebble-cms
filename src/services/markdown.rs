use ammonia::Builder;
use once_cell::sync::Lazy;
use pulldown_cmark::{html, Options, Parser};
use regex::Regex;
use std::collections::HashMap;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

// Statically compiled regexes - avoids runtime panic and improves performance
static SHORTCODE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[(\w+)([^\]]*)\]").expect("Invalid shortcode regex pattern")
});
static ATTR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(\w+)(?:="([^"]*)")?|(\w+)"#).expect("Invalid attribute regex pattern")
});

/// Shortcode processor for embedding media and other dynamic content.
///
/// Supported shortcodes:
/// - `[media src="filename.jpg"]` - Auto-detects type and embeds appropriately
/// - `[image src="filename.jpg" alt="description"]` - Embeds image with optional alt text
/// - `[video src="filename.mp4" controls]` - Embeds video player
/// - `[audio src="filename.mp3" controls]` - Embeds audio player
/// - `[gallery src="file1.jpg,file2.jpg,file3.jpg"]` - Embeds a gallery of images
pub struct ShortcodeProcessor;

impl Default for ShortcodeProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl ShortcodeProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Process all shortcodes in the content and return the processed content.
    pub fn process(&self, content: &str) -> String {
        SHORTCODE_REGEX
            .replace_all(content, |caps: &regex::Captures| {
                let name = &caps[1];
                let attrs_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let attrs = self.parse_attributes(attrs_str);

                match name {
                    "media" => self.render_media(&attrs),
                    "image" | "img" => self.render_image(&attrs),
                    "video" => self.render_video(&attrs),
                    "audio" => self.render_audio(&attrs),
                    "gallery" => self.render_gallery(&attrs),
                    _ => caps[0].to_string(), // Unknown shortcode, leave as-is
                }
            })
            .to_string()
    }

    fn parse_attributes(&self, attrs_str: &str) -> HashMap<String, String> {
        let mut attrs = HashMap::new();

        for cap in ATTR_REGEX.captures_iter(attrs_str) {
            if let Some(name) = cap.get(1) {
                let value = cap.get(2).map(|m| m.as_str()).unwrap_or("true");
                attrs.insert(name.as_str().to_string(), value.to_string());
            } else if let Some(flag) = cap.get(3) {
                attrs.insert(flag.as_str().to_string(), "true".to_string());
            }
        }

        attrs
    }

    fn render_media(&self, attrs: &HashMap<String, String>) -> String {
        let Some(raw_src) = attrs.get("src") else {
            return "<!-- media shortcode: missing src attribute -->".to_string();
        };
        let src = Self::normalize_src(raw_src);

        // Determine type from extension
        let extension = src.rsplit('.').next().unwrap_or("").to_lowercase();

        match extension.as_str() {
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" => self.render_image(attrs),
            "mp4" | "webm" => self.render_video(attrs),
            "mp3" | "ogg" => self.render_audio(attrs),
            "pdf" => self.render_pdf(attrs),
            _ => format!(
                r#"<a href="/media/{}" class="media-link">{}</a>"#,
                html_escape(src),
                html_escape(attrs.get("title").map(|s| s.as_str()).unwrap_or(src))
            ),
        }
    }

    /// Normalize the src path by removing any leading /media/ prefix
    fn normalize_src(src: &str) -> &str {
        src.trim_start_matches("/media/")
            .trim_start_matches("media/")
    }

    fn render_image(&self, attrs: &HashMap<String, String>) -> String {
        let Some(raw_src) = attrs.get("src") else {
            return "<!-- image shortcode: missing src attribute -->".to_string();
        };
        let src = Self::normalize_src(raw_src);

        let alt = attrs.get("alt").map(|s| s.as_str()).unwrap_or("");
        let title = attrs.get("title").map(|s| s.as_str());
        let class = attrs
            .get("class")
            .map(|s| s.as_str())
            .unwrap_or("media-image");
        let width = attrs.get("width");
        let height = attrs.get("height");

        // Build srcset for responsive images (if webp variants exist)
        let base_name = src.rsplit_once('.').map(|(n, _)| n).unwrap_or(src);
        let webp_src = format!("{}.webp", base_name);

        let mut img_attrs = vec![
            format!(r#"src="/media/{}""#, html_escape(src)),
            format!(r#"alt="{}""#, html_escape(alt)),
            format!(r#"class="{}""#, html_escape(class)),
            "loading=\"lazy\"".to_string(),
        ];

        if let Some(t) = title {
            img_attrs.push(format!(r#"title="{}""#, html_escape(t)));
        }
        if let Some(w) = width {
            img_attrs.push(format!(r#"width="{}""#, html_escape(w)));
        }
        if let Some(h) = height {
            img_attrs.push(format!(r#"height="{}""#, html_escape(h)));
        }

        // Use picture element for webp fallback
        format!(
            r#"<figure class="media-figure">
<picture>
<source srcset="/media/{}" type="image/webp">
<img {}>
</picture>
{}</figure>"#,
            html_escape(&webp_src),
            img_attrs.join(" "),
            if !alt.is_empty() {
                format!(r#"<figcaption>{}</figcaption>"#, html_escape(alt))
            } else {
                String::new()
            }
        )
    }

    fn render_video(&self, attrs: &HashMap<String, String>) -> String {
        let Some(raw_src) = attrs.get("src") else {
            return "<!-- video shortcode: missing src attribute -->".to_string();
        };
        let src = Self::normalize_src(raw_src);

        let controls = attrs.contains_key("controls") || !attrs.contains_key("nocontrols");
        let autoplay = attrs.contains_key("autoplay");
        let loop_attr = attrs.contains_key("loop");
        let muted = attrs.contains_key("muted") || autoplay; // Autoplay requires muted
        let class = attrs
            .get("class")
            .map(|s| s.as_str())
            .unwrap_or("media-video");
        let poster = attrs.get("poster");
        let width = attrs.get("width");
        let height = attrs.get("height");

        let mut video_attrs = vec![
            format!(r#"class="{}""#, html_escape(class)),
            "preload=\"metadata\"".to_string(),
        ];

        if controls {
            video_attrs.push("controls".to_string());
        }
        if autoplay {
            video_attrs.push("autoplay".to_string());
        }
        if loop_attr {
            video_attrs.push("loop".to_string());
        }
        if muted {
            video_attrs.push("muted".to_string());
        }
        if let Some(p) = poster {
            video_attrs.push(format!(r#"poster="/media/{}""#, html_escape(p)));
        }
        if let Some(w) = width {
            video_attrs.push(format!(r#"width="{}""#, html_escape(w)));
        }
        if let Some(h) = height {
            video_attrs.push(format!(r#"height="{}""#, html_escape(h)));
        }

        // Determine MIME type from extension
        let extension = src.rsplit('.').next().unwrap_or("").to_lowercase();
        let mime_type = match extension.as_str() {
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            _ => "video/mp4",
        };

        format!(
            r#"<figure class="media-figure">
<video {}>
<source src="/media/{}" type="{}">
Your browser does not support the video tag.
</video>
</figure>"#,
            video_attrs.join(" "),
            html_escape(src),
            mime_type
        )
    }

    fn render_audio(&self, attrs: &HashMap<String, String>) -> String {
        let Some(raw_src) = attrs.get("src") else {
            return "<!-- audio shortcode: missing src attribute -->".to_string();
        };
        let src = Self::normalize_src(raw_src);

        let controls = attrs.contains_key("controls") || !attrs.contains_key("nocontrols");
        let autoplay = attrs.contains_key("autoplay");
        let loop_attr = attrs.contains_key("loop");
        let class = attrs
            .get("class")
            .map(|s| s.as_str())
            .unwrap_or("media-audio");

        let mut audio_attrs = vec![format!(r#"class="{}""#, html_escape(class))];

        if controls {
            audio_attrs.push("controls".to_string());
        }
        if autoplay {
            audio_attrs.push("autoplay".to_string());
        }
        if loop_attr {
            audio_attrs.push("loop".to_string());
        }

        // Determine MIME type from extension
        let extension = src.rsplit('.').next().unwrap_or("").to_lowercase();
        let mime_type = match extension.as_str() {
            "mp3" => "audio/mpeg",
            "ogg" => "audio/ogg",
            _ => "audio/mpeg",
        };

        format!(
            r#"<figure class="media-figure">
<audio {}>
<source src="/media/{}" type="{}">
Your browser does not support the audio tag.
</audio>
</figure>"#,
            audio_attrs.join(" "),
            html_escape(src),
            mime_type
        )
    }

    fn render_pdf(&self, attrs: &HashMap<String, String>) -> String {
        let Some(raw_src) = attrs.get("src") else {
            return "<!-- pdf shortcode: missing src attribute -->".to_string();
        };
        let src = Self::normalize_src(raw_src);

        let width = attrs.get("width").map(|s| s.as_str()).unwrap_or("100%");
        let height = attrs.get("height").map(|s| s.as_str()).unwrap_or("600px");
        let title = attrs
            .get("title")
            .map(|s| s.as_str())
            .unwrap_or("PDF Document");

        format!(
            r#"<figure class="media-figure media-pdf">
<iframe src="/media/{}" width="{}" height="{}" title="{}" class="media-pdf-embed">
<p>Your browser does not support PDFs. <a href="/media/{}">Download the PDF</a>.</p>
</iframe>
</figure>"#,
            html_escape(src),
            html_escape(width),
            html_escape(height),
            html_escape(title),
            html_escape(src)
        )
    }

    fn render_gallery(&self, attrs: &HashMap<String, String>) -> String {
        let Some(raw_src) = attrs.get("src") else {
            return "<!-- gallery shortcode: missing src attribute -->".to_string();
        };

        let class = attrs
            .get("class")
            .map(|s| s.as_str())
            .unwrap_or("media-gallery");
        let columns = attrs.get("columns").map(|s| s.as_str()).unwrap_or("3");

        let images: Vec<String> = raw_src
            .split(',')
            .map(|s| Self::normalize_src(s.trim()).to_string())
            .collect();

        let mut gallery_html = format!(
            r#"<div class="{}" style="display: grid; grid-template-columns: repeat({}, 1fr); gap: 1rem;">"#,
            html_escape(class),
            html_escape(columns)
        );

        for image in &images {
            let base_name = image.rsplit_once('.').map(|(n, _)| n).unwrap_or(image);
            let webp_src = format!("{}.webp", base_name);
            let thumb_src = format!("{}-thumb.webp", base_name);

            gallery_html.push_str(&format!(
                r#"
<a href="/media/{}" class="gallery-item">
<picture>
<source srcset="/media/{}" type="image/webp">
<img src="/media/{}" alt="" loading="lazy" class="gallery-image">
</picture>
</a>"#,
                html_escape(image),
                html_escape(&thumb_src),
                html_escape(&webp_src),
            ));
        }

        gallery_html.push_str("\n</div>");
        gallery_html
    }
}

pub struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    sanitizer: Builder<'static>,
    shortcode_processor: ShortcodeProcessor,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        let mut tags = ammonia::Builder::default().clone_tags();
        tags.insert("pre");
        tags.insert("code");
        tags.insert("span");
        tags.insert("table");
        tags.insert("thead");
        tags.insert("tbody");
        tags.insert("tr");
        tags.insert("th");
        tags.insert("td");
        tags.insert("del");
        tags.insert("input");
        // Media embedding tags
        tags.insert("figure");
        tags.insert("figcaption");
        tags.insert("picture");
        tags.insert("source");
        tags.insert("video");
        tags.insert("audio");
        tags.insert("iframe");

        let mut attrs = ammonia::Builder::default().clone_tag_attributes();
        attrs.insert("span", ["style"].iter().cloned().collect());
        attrs.insert(
            "input",
            ["type", "checked", "disabled"].iter().cloned().collect(),
        );
        // Allow id attributes on headings for TOC anchor links
        attrs.insert("h1", ["id"].iter().cloned().collect());
        attrs.insert("h2", ["id"].iter().cloned().collect());
        attrs.insert("h3", ["id"].iter().cloned().collect());
        attrs.insert("h4", ["id"].iter().cloned().collect());
        attrs.insert("h5", ["id"].iter().cloned().collect());
        attrs.insert("h6", ["id"].iter().cloned().collect());
        // Media element attributes (class is handled via add_allowed_classes)
        attrs.insert(
            "img",
            ["src", "alt", "title", "width", "height", "loading"]
                .iter()
                .cloned()
                .collect(),
        );
        attrs.insert(
            "source",
            ["src", "srcset", "type", "media"].iter().cloned().collect(),
        );
        attrs.insert(
            "video",
            [
                "src", "controls", "autoplay", "loop", "muted", "poster", "width", "height",
                "preload",
            ]
            .iter()
            .cloned()
            .collect(),
        );
        attrs.insert(
            "audio",
            ["src", "controls", "autoplay", "loop"]
                .iter()
                .cloned()
                .collect(),
        );
        attrs.insert(
            "iframe",
            ["src", "width", "height", "title"]
                .iter()
                .cloned()
                .collect(),
        );
        attrs.insert("div", ["style"].iter().cloned().collect());

        let mut sanitizer = Builder::default();
        sanitizer
            .tags(tags)
            .tag_attributes(attrs)
            .add_allowed_classes(
                "code",
                &[
                    "language-rust",
                    "language-python",
                    "language-javascript",
                    "language-typescript",
                    "language-go",
                    "language-c",
                    "language-cpp",
                    "language-java",
                    "language-html",
                    "language-css",
                    "language-json",
                    "language-yaml",
                    "language-toml",
                    "language-sql",
                    "language-bash",
                    "language-shell",
                    "language-markdown",
                ],
            )
            .add_allowed_classes("pre", &["code-block"])
            .add_allowed_classes("figure", &["media-figure", "media-pdf"])
            .add_allowed_classes("img", &["media-image", "gallery-image"])
            .add_allowed_classes("video", &["media-video"])
            .add_allowed_classes("audio", &["media-audio"])
            .add_allowed_classes("iframe", &["media-pdf-embed"])
            .add_allowed_classes("div", &["media-gallery"])
            .add_allowed_classes("a", &["media-link", "gallery-item"])
            .link_rel(Some("noopener noreferrer"));

        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            sanitizer,
            shortcode_processor: ShortcodeProcessor::new(),
        }
    }

    pub fn render(&self, markdown: &str) -> String {
        // Process shortcodes first (before markdown parsing)
        let processed = self.shortcode_processor.process(markdown);

        let options = Options::ENABLE_TABLES
            | Options::ENABLE_FOOTNOTES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_HEADING_ATTRIBUTES;

        let parser = Parser::new_ext(&processed, options);
        let mut events: Vec<pulldown_cmark::Event> = Vec::new();
        let mut in_code_block = false;
        let mut code_lang = String::new();
        let mut code_content = String::new();

        // Track heading state for adding IDs
        let mut in_heading = false;
        let mut heading_text = String::new();

        for event in parser {
            match event {
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    code_lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                        _ => String::new(),
                    };
                    code_content.clear();
                }
                pulldown_cmark::Event::End(pulldown_cmark::TagEnd::CodeBlock) => {
                    in_code_block = false;
                    let highlighted = self.highlight_code(&code_content, &code_lang);
                    events.push(pulldown_cmark::Event::Html(highlighted.into()));
                }
                pulldown_cmark::Event::Text(text) if in_code_block => {
                    code_content.push_str(&text);
                }
                // Handle headings to add ID attributes
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading { level, id, classes, attrs }) => {
                    in_heading = true;
                    heading_text.clear();
                    // If the heading already has an ID from {#custom-id} syntax, use it
                    if id.is_some() {
                        events.push(pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading { level, id, classes, attrs }));
                        in_heading = false; // Don't process further, it already has an ID
                    }
                }
                pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Heading(level)) => {
                    if in_heading {
                        // Generate slug from heading text
                        let slug = slugify(&heading_text);
                        let level_num = match level {
                            pulldown_cmark::HeadingLevel::H1 => 1,
                            pulldown_cmark::HeadingLevel::H2 => 2,
                            pulldown_cmark::HeadingLevel::H3 => 3,
                            pulldown_cmark::HeadingLevel::H4 => 4,
                            pulldown_cmark::HeadingLevel::H5 => 5,
                            pulldown_cmark::HeadingLevel::H6 => 6,
                        };
                        // Emit heading with ID as raw HTML
                        let heading_html = format!(
                            r#"<h{} id="{}">{}</h{}>"#,
                            level_num,
                            html_escape(&slug),
                            html_escape(&heading_text),
                            level_num
                        );
                        events.push(pulldown_cmark::Event::Html(heading_html.into()));
                        in_heading = false;
                    } else {
                        events.push(pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Heading(level)));
                    }
                }
                pulldown_cmark::Event::Text(text) if in_heading => {
                    heading_text.push_str(&text);
                }
                _ => events.push(event),
            }
        }

        let mut html_output = String::new();
        html::push_html(&mut html_output, events.into_iter());

        self.sanitizer.clean(&html_output).to_string()
    }

    fn highlight_code(&self, code: &str, lang: &str) -> String {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes["base16-ocean.dark"];

        match highlighted_html_for_string(code, &self.syntax_set, syntax, theme) {
            Ok(html) => {
                // syntect outputs <pre style="..."><span>...</span></pre>
                // We need to strip the outer <pre> and just use the inner content
                let inner = html
                    .trim()
                    .strip_prefix("<pre style=\"background-color:#2b303b;\">\n")
                    .and_then(|s| s.strip_suffix("\n</pre>"))
                    .or_else(|| {
                        html.trim()
                            .strip_prefix("<pre style=\"background-color:#2b303b;\">")
                            .and_then(|s| s.strip_suffix("</pre>"))
                    })
                    .unwrap_or(&html);
                format!(
                    r#"<pre class="code-block"><code class="language-{}">{}</code></pre>"#,
                    lang, inner
                )
            }
            Err(_) => format!(
                r#"<pre class="code-block"><code class="language-{}">{}</code></pre>"#,
                lang,
                html_escape(code)
            ),
        }
    }

    pub fn generate_excerpt(&self, markdown: &str, max_len: usize) -> String {
        let text: String = markdown
            .lines()
            .filter(|line| !line.starts_with('#') && !line.starts_with("```") && !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        let text = strip_markdown(&text);

        let char_count = text.chars().count();
        if char_count <= max_len {
            text
        } else {
            let truncated: String = text.chars().take(max_len).collect();
            let last_space_pos = truncated
                .char_indices()
                .rev()
                .find(|(_, c)| *c == ' ')
                .map(|(i, _)| i);

            if let Some(pos) = last_space_pos {
                format!("{}...", &truncated[..pos])
            } else {
                format!("{}...", truncated)
            }
        }
    }

    /// Calculate estimated reading time in minutes based on word count.
    /// Uses 200 words per minute as average reading speed.
    pub fn calculate_reading_time(&self, markdown: &str) -> u32 {
        let word_count = markdown
            .split_whitespace()
            .filter(|word| !word.starts_with('#') && !word.starts_with("```"))
            .count();

        // 200 words per minute, minimum 1 minute
        ((word_count as f64 / 200.0).ceil() as u32).max(1)
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Convert text to a URL-friendly slug for heading IDs
fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' || c == '_' {
                '-'
            } else {
                // Skip other characters
                '\0'
            }
        })
        .filter(|&c| c != '\0')
        .collect::<String>()
        // Collapse multiple dashes
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn strip_markdown(text: &str) -> String {
    let mut result = text.to_string();

    // Remove inline code
    while let Some(start) = result.find('`') {
        if let Some(end) = result[start + 1..].find('`') {
            let code_content = &result[start + 1..start + 1 + end];
            result = format!(
                "{}{}{}",
                &result[..start],
                code_content,
                &result[start + 2 + end..]
            );
        } else {
            break;
        }
    }

    // Remove links [text](url) -> text
    while let Some(bracket_start) = result.find('[') {
        if let Some(bracket_end) = result[bracket_start..].find("](") {
            let abs_bracket_end = bracket_start + bracket_end;
            if let Some(paren_end) = result[abs_bracket_end + 2..].find(')') {
                let link_text = &result[bracket_start + 1..abs_bracket_end];
                result = format!(
                    "{}{}{}",
                    &result[..bracket_start],
                    link_text,
                    &result[abs_bracket_end + 3 + paren_end..]
                );
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Remove bold/italic markers
    result = result.replace("***", "");
    result = result.replace("**", "");
    result = result.replace("__", "");
    result = result.replace('*', "");
    result = result.replace('_', " ");

    // Remove images ![alt](url)
    while let Some(img_start) = result.find("![") {
        if let Some(bracket_end) = result[img_start + 2..].find("](") {
            let abs_bracket_end = img_start + 2 + bracket_end;
            if let Some(paren_end) = result[abs_bracket_end + 2..].find(')') {
                result = format!(
                    "{}{}",
                    &result[..img_start],
                    &result[abs_bracket_end + 3 + paren_end..]
                );
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Clean up multiple spaces
    while result.contains("  ") {
        result = result.replace("  ", " ");
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Quick Start"), "quick-start");
        assert_eq!(slugify("CLI Commands"), "cli-commands");
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("Test  Multiple   Spaces"), "test-multiple-spaces");
        assert_eq!(slugify("Already-Hyphenated"), "already-hyphenated");
    }

    #[test]
    fn test_heading_ids() {
        let renderer = MarkdownRenderer::new();
        let input = "## Quick Start\n\nSome content here.";
        let output = renderer.render(input);
        assert!(output.contains(r#"id="quick-start""#), "Output was: {}", output);
    }

    #[test]
    fn test_toc_links_match_headings() {
        let renderer = MarkdownRenderer::new();
        let input = r#"## Table of Contents

- [Quick Start](#quick-start)
- [CLI Commands](#cli-commands)

## Quick Start

Getting started guide.

## CLI Commands

Command reference.
"#;
        let output = renderer.render(input);
        // Check that heading IDs match TOC links
        assert!(output.contains(r#"id="quick-start""#), "Missing quick-start ID. Output: {}", output);
        assert!(output.contains(r#"id="cli-commands""#), "Missing cli-commands ID. Output: {}", output);
        assert!(output.contains("href=\"#quick-start\""), "Missing quick-start link. Output: {}", output);
    }
}
