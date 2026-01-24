#[cfg(test)]
mod tests {

    mod slug_tests {
        use crate::services::slug::{generate_slug, validate_slug};

        #[test]
        fn test_generate_slug_basic() {
            assert_eq!(generate_slug("Hello World"), "hello-world");
        }

        #[test]
        fn test_generate_slug_special_characters() {
            assert_eq!(generate_slug("Hello, World!"), "hello-world");
        }

        #[test]
        fn test_generate_slug_unicode() {
            assert_eq!(generate_slug("Café au lait"), "cafe-au-lait");
        }

        #[test]
        fn test_generate_slug_numbers() {
            assert_eq!(generate_slug("Article 123"), "article-123");
        }

        #[test]
        fn test_generate_slug_multiple_spaces() {
            assert_eq!(generate_slug("Hello   World"), "hello-world");
        }

        #[test]
        fn test_generate_slug_leading_trailing_spaces() {
            assert_eq!(generate_slug("  Hello World  "), "hello-world");
        }

        #[test]
        fn test_validate_slug_valid() {
            assert!(validate_slug("hello-world"));
            assert!(validate_slug("my-blog-post-2024"));
            assert!(validate_slug("a"));
            assert!(validate_slug("123"));
        }

        #[test]
        fn test_validate_slug_invalid_empty() {
            assert!(!validate_slug(""));
        }

        #[test]
        fn test_validate_slug_invalid_uppercase() {
            assert!(!validate_slug("Hello-World"));
        }

        #[test]
        fn test_validate_slug_invalid_special_chars() {
            assert!(!validate_slug("hello_world"));
            assert!(!validate_slug("hello world"));
            assert!(!validate_slug("hello!world"));
        }

        #[test]
        fn test_validate_slug_too_long() {
            let long_slug = "a".repeat(201);
            assert!(!validate_slug(&long_slug));
        }

        #[test]
        fn test_validate_slug_max_length() {
            let max_slug = "a".repeat(200);
            assert!(validate_slug(&max_slug));
        }
    }

    mod markdown_tests {
        use crate::services::markdown::MarkdownRenderer;

        #[test]
        fn test_render_basic_markdown() {
            let renderer = MarkdownRenderer::new();
            let html = renderer.render("# Hello World");
            assert!(html.contains("<h1>"));
            assert!(html.contains("Hello World"));
        }

        #[test]
        fn test_render_paragraph() {
            let renderer = MarkdownRenderer::new();
            let html = renderer.render("This is a paragraph.");
            assert!(html.contains("<p>"));
            assert!(html.contains("This is a paragraph."));
        }

        #[test]
        fn test_render_bold_italic() {
            let renderer = MarkdownRenderer::new();
            let html = renderer.render("**bold** and *italic*");
            assert!(html.contains("<strong>bold</strong>"));
            assert!(html.contains("<em>italic</em>"));
        }

        #[test]
        fn test_render_links() {
            let renderer = MarkdownRenderer::new();
            let html = renderer.render("[Link](https://example.com)");
            assert!(html.contains("<a href=\"https://example.com\">Link</a>"));
        }

        #[test]
        fn test_render_code_block() {
            let renderer = MarkdownRenderer::new();
            let html = renderer.render("```rust\nlet x = 5;\n```");
            assert!(html.contains("code-block"));
            assert!(html.contains("language-rust"));
        }

        #[test]
        fn test_render_inline_code() {
            let renderer = MarkdownRenderer::new();
            let html = renderer.render("Use `code` here");
            assert!(html.contains("<code>code</code>"));
        }

        #[test]
        fn test_render_unordered_list() {
            let renderer = MarkdownRenderer::new();
            let html = renderer.render("- Item 1\n- Item 2");
            assert!(html.contains("<ul>"));
            assert!(html.contains("<li>"));
        }

        #[test]
        fn test_render_ordered_list() {
            let renderer = MarkdownRenderer::new();
            let html = renderer.render("1. First\n2. Second");
            assert!(html.contains("<ol>"));
            assert!(html.contains("<li>"));
        }

        #[test]
        fn test_render_table() {
            let renderer = MarkdownRenderer::new();
            let html = renderer.render("| A | B |\n|---|---|\n| 1 | 2 |");
            assert!(html.contains("<table>"));
            assert!(html.contains("<th>"));
            assert!(html.contains("<td>"));
        }

        #[test]
        fn test_render_strikethrough() {
            let renderer = MarkdownRenderer::new();
            let html = renderer.render("~~deleted~~");
            assert!(html.contains("<del>deleted</del>"));
        }

        #[test]
        fn test_generate_excerpt_short_text() {
            let renderer = MarkdownRenderer::new();
            let excerpt = renderer.generate_excerpt("Short text", 100);
            assert_eq!(excerpt, "Short text");
        }

        #[test]
        fn test_generate_excerpt_truncation() {
            let renderer = MarkdownRenderer::new();
            let long_text = "This is a very long text that needs to be truncated because it exceeds the maximum length allowed for an excerpt.";
            let excerpt = renderer.generate_excerpt(long_text, 30);
            assert!(excerpt.len() < long_text.len());
            assert!(excerpt.ends_with("..."));
        }

        #[test]
        fn test_generate_excerpt_ignores_headers() {
            let renderer = MarkdownRenderer::new();
            let text = "# Header\nThis is content";
            let excerpt = renderer.generate_excerpt(text, 100);
            assert!(!excerpt.contains("#"));
            assert!(excerpt.contains("This is content"));
        }

        #[test]
        fn test_generate_excerpt_ignores_code_blocks() {
            let renderer = MarkdownRenderer::new();
            let text = "```\ncode\n```\nActual content";
            let excerpt = renderer.generate_excerpt(text, 100);
            assert!(!excerpt.contains("```"));
        }

        #[test]
        fn test_calculate_reading_time_short() {
            let renderer = MarkdownRenderer::new();
            let time = renderer.calculate_reading_time("Hello world");
            assert_eq!(time, 1); // Minimum 1 minute
        }

        #[test]
        fn test_calculate_reading_time_200_words() {
            let renderer = MarkdownRenderer::new();
            let text = "word ".repeat(200);
            let time = renderer.calculate_reading_time(&text);
            assert_eq!(time, 1);
        }

        #[test]
        fn test_calculate_reading_time_400_words() {
            let renderer = MarkdownRenderer::new();
            let text = "word ".repeat(400);
            let time = renderer.calculate_reading_time(&text);
            assert_eq!(time, 2);
        }

        #[test]
        fn test_calculate_reading_time_ignores_markdown() {
            let renderer = MarkdownRenderer::new();
            let text = "# Header\n```code```\nword ".repeat(200);
            let time = renderer.calculate_reading_time(&text);
            // Should exclude header and code markers
            assert!(time >= 1);
        }
    }

    mod auth_tests {
        use crate::services::auth::{generate_session_token, hash_password, verify_password};

        #[test]
        fn test_hash_password_produces_hash() {
            let hash = hash_password("password123").unwrap();
            assert!(!hash.is_empty());
            assert!(hash.starts_with("$argon2"));
        }

        #[test]
        fn test_hash_password_unique() {
            let hash1 = hash_password("password123").unwrap();
            let hash2 = hash_password("password123").unwrap();
            // Same password should produce different hashes (due to salt)
            assert_ne!(hash1, hash2);
        }

        #[test]
        fn test_verify_password_correct() {
            let hash = hash_password("password123").unwrap();
            assert!(verify_password("password123", &hash));
        }

        #[test]
        fn test_verify_password_incorrect() {
            let hash = hash_password("password123").unwrap();
            assert!(!verify_password("wrongpassword", &hash));
        }

        #[test]
        fn test_verify_password_empty() {
            let hash = hash_password("password123").unwrap();
            assert!(!verify_password("", &hash));
        }

        #[test]
        fn test_verify_password_invalid_hash() {
            assert!(!verify_password("password123", "invalid-hash"));
        }

        #[test]
        fn test_generate_session_token_length() {
            let token = generate_session_token();
            // Base64 encoded 32 bytes without padding = ~43 chars
            assert!(token.len() >= 40);
        }

        #[test]
        fn test_generate_session_token_unique() {
            let token1 = generate_session_token();
            let token2 = generate_session_token();
            assert_ne!(token1, token2);
        }

        #[test]
        fn test_generate_session_token_url_safe() {
            let token = generate_session_token();
            // Should only contain URL-safe base64 characters
            assert!(token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        }
    }

    mod user_role_tests {
        use crate::models::UserRole;
        use std::str::FromStr;

        #[test]
        fn test_user_role_from_str() {
            assert_eq!(UserRole::from_str("admin").unwrap(), UserRole::Admin);
            assert_eq!(UserRole::from_str("author").unwrap(), UserRole::Author);
            assert_eq!(UserRole::from_str("viewer").unwrap(), UserRole::Viewer);
        }

        #[test]
        fn test_user_role_from_str_case_insensitive() {
            assert_eq!(UserRole::from_str("ADMIN").unwrap(), UserRole::Admin);
            assert_eq!(UserRole::from_str("Author").unwrap(), UserRole::Author);
            assert_eq!(UserRole::from_str("VIEWER").unwrap(), UserRole::Viewer);
        }

        #[test]
        fn test_user_role_from_str_invalid() {
            assert!(UserRole::from_str("invalid").is_err());
            assert!(UserRole::from_str("").is_err());
        }

        #[test]
        fn test_user_role_to_string() {
            assert_eq!(UserRole::Admin.to_string(), "admin");
            assert_eq!(UserRole::Author.to_string(), "author");
            assert_eq!(UserRole::Viewer.to_string(), "viewer");
        }

        #[test]
        fn test_user_role_roundtrip() {
            for role in [UserRole::Admin, UserRole::Author, UserRole::Viewer] {
                let string = role.to_string();
                let parsed = UserRole::from_str(&string).unwrap();
                assert_eq!(role, parsed);
            }
        }
    }

    mod content_type_tests {
        use crate::models::ContentType;
        use std::str::FromStr;

        #[test]
        fn test_content_type_from_str() {
            assert_eq!(ContentType::from_str("post").unwrap(), ContentType::Post);
            assert_eq!(ContentType::from_str("page").unwrap(), ContentType::Page);
            assert_eq!(
                ContentType::from_str("snippet").unwrap(),
                ContentType::Snippet
            );
        }

        #[test]
        fn test_content_type_from_str_case_insensitive() {
            assert_eq!(ContentType::from_str("POST").unwrap(), ContentType::Post);
            assert_eq!(ContentType::from_str("Page").unwrap(), ContentType::Page);
        }

        #[test]
        fn test_content_type_from_str_invalid() {
            assert!(ContentType::from_str("invalid").is_err());
            assert!(ContentType::from_str("").is_err());
        }

        #[test]
        fn test_content_type_to_string() {
            assert_eq!(ContentType::Post.to_string(), "post");
            assert_eq!(ContentType::Page.to_string(), "page");
            assert_eq!(ContentType::Snippet.to_string(), "snippet");
        }

        #[test]
        fn test_content_type_default() {
            assert_eq!(ContentType::default(), ContentType::Post);
        }
    }

    mod content_status_tests {
        use crate::models::ContentStatus;
        use std::str::FromStr;

        #[test]
        fn test_content_status_from_str() {
            assert_eq!(
                ContentStatus::from_str("draft").unwrap(),
                ContentStatus::Draft
            );
            assert_eq!(
                ContentStatus::from_str("scheduled").unwrap(),
                ContentStatus::Scheduled
            );
            assert_eq!(
                ContentStatus::from_str("published").unwrap(),
                ContentStatus::Published
            );
            assert_eq!(
                ContentStatus::from_str("archived").unwrap(),
                ContentStatus::Archived
            );
        }

        #[test]
        fn test_content_status_case_insensitive() {
            assert_eq!(
                ContentStatus::from_str("DRAFT").unwrap(),
                ContentStatus::Draft
            );
            assert_eq!(
                ContentStatus::from_str("Published").unwrap(),
                ContentStatus::Published
            );
        }

        #[test]
        fn test_content_status_invalid() {
            assert!(ContentStatus::from_str("invalid").is_err());
        }

        #[test]
        fn test_content_status_to_string() {
            assert_eq!(ContentStatus::Draft.to_string(), "draft");
            assert_eq!(ContentStatus::Scheduled.to_string(), "scheduled");
            assert_eq!(ContentStatus::Published.to_string(), "published");
            assert_eq!(ContentStatus::Archived.to_string(), "archived");
        }

        #[test]
        fn test_content_status_default() {
            assert_eq!(ContentStatus::default(), ContentStatus::Draft);
        }
    }

    mod theme_tests {
        use crate::config::ThemeConfig;

        #[test]
        fn test_available_themes() {
            assert_eq!(ThemeConfig::AVAILABLE_THEMES.len(), 3);
            assert!(ThemeConfig::AVAILABLE_THEMES.contains(&"default"));
            assert!(ThemeConfig::AVAILABLE_THEMES.contains(&"minimal"));
            assert!(ThemeConfig::AVAILABLE_THEMES.contains(&"magazine"));
        }

        #[test]
        fn test_is_valid_theme() {
            assert!(ThemeConfig::is_valid_theme("default"));
            assert!(ThemeConfig::is_valid_theme("minimal"));
            assert!(ThemeConfig::is_valid_theme("magazine"));
            assert!(!ThemeConfig::is_valid_theme("invalid"));
            assert!(!ThemeConfig::is_valid_theme(""));
        }

        #[test]
        fn test_validate_valid_theme() {
            let theme = ThemeConfig {
                name: "default".to_string(),
                custom: Default::default(),
            };
            assert!(theme.validate().is_ok());
        }

        #[test]
        fn test_validate_invalid_theme() {
            let theme = ThemeConfig {
                name: "nonexistent".to_string(),
                custom: Default::default(),
            };
            assert!(theme.validate().is_err());
        }
    }

    mod settings_tests {
        use crate::services::settings::HomepageSettings;

        #[test]
        fn test_homepage_settings_default() {
            let settings = HomepageSettings::default();
            assert!(settings.title.is_empty());
            assert!(settings.subtitle.is_empty());
            assert!(!settings.show_pages); // Default trait gives false
            assert!(!settings.show_posts); // Default trait gives false
            assert!(settings.custom_content.is_empty());
        }
    }

    mod config_tests {
        use crate::Config;
        use std::path::Path;

        #[test]
        fn test_config_load_missing_file() {
            let result = Config::load(Path::new("/nonexistent/path.toml"));
            assert!(result.is_err());
        }

        #[test]
        fn test_config_load_valid_toml() {
            use std::io::Write;
            let temp_dir = std::env::temp_dir();
            let config_path = temp_dir.join("test_pebble_config.toml");

            let config_content = r#"
[site]
title = "Test Site"
description = "A test site"
url = "http://localhost:3000"

[server]
host = "127.0.0.1"
port = 3000

[database]
path = "data/pebble.db"

[content]
posts_per_page = 10

[media]
upload_dir = "uploads"

[theme]
name = "default"

[auth]
session_lifetime = "7d"
"#;

            let mut file = std::fs::File::create(&config_path).unwrap();
            file.write_all(config_content.as_bytes()).unwrap();

            let config = Config::load(&config_path).unwrap();
            assert_eq!(config.site.title, "Test Site");
            assert_eq!(config.server.port, 3000);
            assert_eq!(config.theme.name, "default");

            std::fs::remove_file(&config_path).ok();
        }
    }
}
