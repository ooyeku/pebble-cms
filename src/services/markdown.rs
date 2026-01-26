use ammonia::Builder;
use pulldown_cmark::{html, Options, Parser};
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

pub struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    sanitizer: Builder<'static>,
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

        let mut attrs = ammonia::Builder::default().clone_tag_attributes();
        attrs.insert("span", ["style"].iter().cloned().collect());
        attrs.insert(
            "input",
            ["type", "checked", "disabled"].iter().cloned().collect(),
        );

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
            .link_rel(Some("noopener noreferrer"));

        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            sanitizer,
        }
    }

    pub fn render(&self, markdown: &str) -> String {
        let options = Options::ENABLE_TABLES
            | Options::ENABLE_FOOTNOTES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS;

        let parser = Parser::new_ext(markdown, options);
        let mut events: Vec<pulldown_cmark::Event> = Vec::new();
        let mut in_code_block = false;
        let mut code_lang = String::new();
        let mut code_content = String::new();

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
            Ok(html) => format!(
                r#"<pre class="code-block"><code class="language-{}">{}</code></pre>"#,
                lang, html
            ),
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
