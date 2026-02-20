use pebble_cms::models::{ContentStatus, ContentType, UserRole};
use pebble_cms::services::{auth, content, database, search, settings, tags};
use pebble_cms::Database;

fn create_test_db() -> Database {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let id: u32 = rng.gen();
    let name = format!("test_db_{}", id);

    let db = Database::open_memory(&name).expect("Failed to create test database");
    db.migrate().expect("Failed to run migrations");
    db
}

// Valid test passwords that meet requirements: 8+ chars, uppercase, lowercase, number
const TEST_PASSWORD: &str = "Password123";
const WRONG_PASSWORD: &str = "WrongPass456";
const OLD_PASSWORD: &str = "OldPass123";
const NEW_PASSWORD: &str = "NewPass456";

mod auth_integration_tests {
    use super::*;

    #[test]
    fn test_create_and_authenticate_user() {
        let db = create_test_db();

        let user_id = auth::create_user(
            &db,
            "testuser",
            "test@example.com",
            TEST_PASSWORD,
            UserRole::Admin,
        )
        .expect("Failed to create user");

        assert!(user_id > 0);

        let user = auth::authenticate(&db, "testuser", TEST_PASSWORD)
            .expect("Authentication error")
            .expect("User should be found");

        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.role, UserRole::Admin);
    }

    #[test]
    fn test_authenticate_wrong_password() {
        let db = create_test_db();

        auth::create_user(
            &db,
            "testuser",
            "test@example.com",
            TEST_PASSWORD,
            UserRole::Admin,
        )
        .expect("Failed to create user");

        let result =
            auth::authenticate(&db, "testuser", WRONG_PASSWORD).expect("Authentication error");

        assert!(result.is_none());
    }

    #[test]
    fn test_authenticate_nonexistent_user() {
        let db = create_test_db();

        let result =
            auth::authenticate(&db, "nonexistent", TEST_PASSWORD).expect("Authentication error");

        assert!(result.is_none());
    }

    #[test]
    fn test_has_users_empty() {
        let db = create_test_db();
        assert!(!auth::has_users(&db).unwrap());
    }

    #[test]
    fn test_has_users_with_user() {
        let db = create_test_db();
        auth::create_user(
            &db,
            "testuser",
            "test@example.com",
            TEST_PASSWORD,
            UserRole::Admin,
        )
        .unwrap();
        assert!(auth::has_users(&db).unwrap());
    }

    #[test]
    fn test_list_users() {
        let db = create_test_db();

        auth::create_user(
            &db,
            "user1",
            "user1@example.com",
            TEST_PASSWORD,
            UserRole::Admin,
        )
        .unwrap();
        auth::create_user(
            &db,
            "user2",
            "user2@example.com",
            TEST_PASSWORD,
            UserRole::Author,
        )
        .unwrap();

        let users = auth::list_users(&db).unwrap();
        assert_eq!(users.len(), 2);
    }

    #[test]
    fn test_get_user() {
        let db = create_test_db();

        let user_id = auth::create_user(
            &db,
            "testuser",
            "test@example.com",
            TEST_PASSWORD,
            UserRole::Admin,
        )
        .unwrap();

        let user = auth::get_user(&db, user_id)
            .unwrap()
            .expect("User should exist");
        assert_eq!(user.id, user_id);
        assert_eq!(user.username, "testuser");
    }

    #[test]
    fn test_update_user() {
        let db = create_test_db();

        let user_id = auth::create_user(
            &db,
            "testuser",
            "old@example.com",
            TEST_PASSWORD,
            UserRole::Author,
        )
        .unwrap();

        auth::update_user(&db, user_id, Some("new@example.com"), Some(UserRole::Admin)).unwrap();

        let user = auth::get_user(&db, user_id)
            .unwrap()
            .expect("User should exist");
        assert_eq!(user.email, "new@example.com");
        assert_eq!(user.role, UserRole::Admin);
    }

    #[test]
    fn test_delete_user() {
        let db = create_test_db();

        let user_id = auth::create_user(
            &db,
            "testuser",
            "test@example.com",
            TEST_PASSWORD,
            UserRole::Admin,
        )
        .unwrap();

        auth::delete_user(&db, user_id).unwrap();

        let user = auth::get_user(&db, user_id).unwrap();
        assert!(user.is_none());
    }

    #[test]
    fn test_session_lifecycle() {
        let db = create_test_db();

        let user_id = auth::create_user(
            &db,
            "testuser",
            "test@example.com",
            TEST_PASSWORD,
            UserRole::Admin,
        )
        .unwrap();

        let token = auth::create_session(&db, user_id, 7).expect("Failed to create session");
        assert!(!token.is_empty());

        let session_user = auth::validate_session(&db, &token)
            .expect("Session validation failed")
            .expect("Session should be valid");

        assert_eq!(session_user.id, user_id);

        auth::delete_session(&db, &token).expect("Failed to delete session");

        let deleted = auth::validate_session(&db, &token).unwrap();
        assert!(deleted.is_none());
    }

    #[test]
    fn test_update_password() {
        let db = create_test_db();

        auth::create_user(
            &db,
            "testuser",
            "test@example.com",
            OLD_PASSWORD,
            UserRole::Admin,
        )
        .unwrap();

        auth::update_password(&db, "testuser", NEW_PASSWORD).unwrap();

        // Old password should fail
        let old_auth = auth::authenticate(&db, "testuser", OLD_PASSWORD).unwrap();
        assert!(old_auth.is_none());

        // New password should work
        let new_auth = auth::authenticate(&db, "testuser", NEW_PASSWORD).unwrap();
        assert!(new_auth.is_some());
    }
}

mod tag_integration_tests {
    use super::*;

    #[test]
    fn test_create_tag() {
        let db = create_test_db();

        let tag_id = tags::create_tag(&db, "Rust", None).expect("Failed to create tag");
        assert!(tag_id > 0);
    }

    #[test]
    fn test_create_tag_with_custom_slug() {
        let db = create_test_db();

        tags::create_tag(&db, "My Tag", Some("custom-slug")).expect("Failed to create tag");

        let tag = tags::get_tag_by_slug(&db, "custom-slug")
            .unwrap()
            .expect("Tag should exist");
        assert_eq!(tag.name, "My Tag");
        assert_eq!(tag.slug, "custom-slug");
    }

    #[test]
    fn test_get_tag_by_slug() {
        let db = create_test_db();

        tags::create_tag(&db, "Rust Programming", None).unwrap();

        let tag = tags::get_tag_by_slug(&db, "rust-programming").unwrap();
        assert!(tag.is_some());
        assert_eq!(tag.unwrap().name, "Rust Programming");
    }

    #[test]
    fn test_list_tags() {
        let db = create_test_db();

        tags::create_tag(&db, "Tag A", None).unwrap();
        tags::create_tag(&db, "Tag B", None).unwrap();
        tags::create_tag(&db, "Tag C", None).unwrap();

        let all_tags = tags::list_tags(&db).unwrap();
        assert_eq!(all_tags.len(), 3);
    }

    #[test]
    fn test_list_tags_with_counts() {
        let db = create_test_db();

        tags::create_tag(&db, "Tag 1", None).unwrap();
        tags::create_tag(&db, "Tag 2", None).unwrap();

        let tags_with_counts = tags::list_tags_with_counts(&db).unwrap();
        assert_eq!(tags_with_counts.len(), 2);
        // Without any content, counts should be 0
        for tc in tags_with_counts {
            assert_eq!(tc.count, 0);
        }
    }

    #[test]
    fn test_update_tag() {
        let db = create_test_db();

        let tag_id = tags::create_tag(&db, "Old Name", None).unwrap();

        tags::update_tag(&db, tag_id, "New Name", None).unwrap();

        let tag = tags::get_tag_by_slug(&db, "new-name")
            .unwrap()
            .expect("Tag should exist");
        assert_eq!(tag.name, "New Name");
    }

    #[test]
    fn test_delete_tag() {
        let db = create_test_db();

        let tag_id = tags::create_tag(&db, "To Delete", None).unwrap();

        tags::delete_tag(&db, tag_id).unwrap();

        let tag = tags::get_tag_by_slug(&db, "to-delete").unwrap();
        assert!(tag.is_none());
    }
}

mod content_integration_tests {
    use super::*;
    use pebble_cms::models::CreateContent;

    fn create_test_post(title: &str) -> CreateContent {
        CreateContent {
            title: title.to_string(),
            slug: None,
            content_type: ContentType::Post,
            body_markdown: format!("# {}\n\nThis is test content.", title),
            excerpt: None,
            featured_image: None,
            status: ContentStatus::Draft,
            scheduled_at: None,
            tags: vec![],
            metadata: None,
        }
    }

    #[test]
    fn test_create_content() {
        let db = create_test_db();

        let input = create_test_post("Test Post");
        let content_id = content::create_content(&db, input, None, 200).unwrap();

        assert!(content_id > 0);
    }

    #[test]
    fn test_get_content_by_id() {
        let db = create_test_db();

        let input = create_test_post("Test Post");
        let content_id = content::create_content(&db, input, None, 200).unwrap();

        let post = content::get_content_by_id(&db, content_id)
            .unwrap()
            .expect("Content should exist");
        assert_eq!(post.content.title, "Test Post");
        assert_eq!(post.content.slug, "test-post");
    }

    #[test]
    fn test_get_content_by_slug() {
        let db = create_test_db();

        let input = create_test_post("My Blog Post");
        content::create_content(&db, input, None, 200).unwrap();

        let post = content::get_content_by_slug(&db, "my-blog-post")
            .unwrap()
            .expect("Content should exist");

        assert_eq!(post.content.title, "My Blog Post");
    }

    #[test]
    fn test_list_content() {
        let db = create_test_db();

        for i in 1..=5 {
            let input = create_test_post(&format!("Post {}", i));
            content::create_content(&db, input, None, 200).unwrap();
        }

        let posts = content::list_content(&db, Some(ContentType::Post), None, 10, 0).unwrap();
        assert_eq!(posts.len(), 5);
    }

    #[test]
    fn test_list_content_pagination() {
        let db = create_test_db();

        for i in 1..=10 {
            let input = create_test_post(&format!("Post {}", i));
            content::create_content(&db, input, None, 200).unwrap();
        }

        // First page
        let page1 = content::list_content(&db, Some(ContentType::Post), None, 5, 0).unwrap();
        assert_eq!(page1.len(), 5);

        // Second page
        let page2 = content::list_content(&db, Some(ContentType::Post), None, 5, 5).unwrap();
        assert_eq!(page2.len(), 5);
    }

    #[test]
    fn test_list_published_content() {
        let db = create_test_db();

        // Create draft post
        let draft = create_test_post("Draft Post");
        content::create_content(&db, draft, None, 200).unwrap();

        // Create published post
        let mut published = create_test_post("Published Post");
        published.status = ContentStatus::Published;
        content::create_content(&db, published, None, 200).unwrap();

        let published_posts =
            content::list_published_content(&db, ContentType::Post, 10, 0).unwrap();
        assert_eq!(published_posts.len(), 1);
        assert_eq!(published_posts[0].content.title, "Published Post");
    }

    #[test]
    fn test_count_content() {
        let db = create_test_db();

        for i in 1..=3 {
            let input = create_test_post(&format!("Post {}", i));
            content::create_content(&db, input, None, 200).unwrap();
        }

        let count = content::count_content(&db, Some(ContentType::Post), None).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_count_content_by_status() {
        let db = create_test_db();

        // Create 2 drafts
        for i in 1..=2 {
            let input = create_test_post(&format!("Draft {}", i));
            content::create_content(&db, input, None, 200).unwrap();
        }

        // Create 1 published
        let mut published = create_test_post("Published");
        published.status = ContentStatus::Published;
        content::create_content(&db, published, None, 200).unwrap();

        let draft_count =
            content::count_content(&db, Some(ContentType::Post), Some(ContentStatus::Draft))
                .unwrap();
        assert_eq!(draft_count, 2);

        let published_count =
            content::count_content(&db, Some(ContentType::Post), Some(ContentStatus::Published))
                .unwrap();
        assert_eq!(published_count, 1);
    }

    #[test]
    fn test_update_content() {
        let db = create_test_db();

        let input = create_test_post("Original Title");
        let content_id = content::create_content(&db, input, None, 200).unwrap();

        let update = pebble_cms::models::UpdateContent {
            title: Some("Updated Title".to_string()),
            slug: None,
            body_markdown: None,
            excerpt: None,
            featured_image: None,
            status: None,
            scheduled_at: None,
            tags: None,
            metadata: None,
        };

        content::update_content(&db, content_id, update, 200, None, 50).unwrap();

        let updated = content::get_content_by_id(&db, content_id)
            .unwrap()
            .expect("Content should exist");
        assert_eq!(updated.content.title, "Updated Title");
    }

    #[test]
    fn test_delete_content() {
        let db = create_test_db();

        let input = create_test_post("To Delete");
        let content_id = content::create_content(&db, input, None, 200).unwrap();

        content::delete_content(&db, content_id).unwrap();

        let deleted = content::get_content_by_id(&db, content_id).unwrap();
        assert!(deleted.is_none());
    }

    #[test]
    fn test_create_page() {
        let db = create_test_db();

        let input = CreateContent {
            title: "About Us".to_string(),
            slug: None,
            content_type: ContentType::Page,
            body_markdown: "# About\n\nWe are awesome.".to_string(),
            excerpt: None,
            featured_image: None,
            status: ContentStatus::Published,
            scheduled_at: None,
            tags: vec![],
            metadata: None,
        };

        content::create_content(&db, input, None, 200).unwrap();

        let page = content::get_content_by_slug(&db, "about-us")
            .unwrap()
            .expect("Page should exist");

        assert_eq!(page.content.title, "About Us");
        assert_eq!(page.content.content_type, ContentType::Page);
    }

    #[test]
    fn test_content_with_tags() {
        let db = create_test_db();

        // Create content with tags (tags will be auto-created)
        let input = CreateContent {
            title: "Tagged Post".to_string(),
            slug: None,
            content_type: ContentType::Post,
            body_markdown: "Content with tags".to_string(),
            excerpt: None,
            featured_image: None,
            status: ContentStatus::Published,
            scheduled_at: None,
            tags: vec!["Rust".to_string(), "Programming".to_string()],
            metadata: None,
        };

        let content_id = content::create_content(&db, input, None, 200).unwrap();

        let post = content::get_content_by_id(&db, content_id)
            .unwrap()
            .expect("Content should exist");
        assert_eq!(post.tags.len(), 2);
    }

    #[test]
    fn test_auto_excerpt_generation() {
        let db = create_test_db();

        let long_content = "# Title\n\n".to_string() + &"This is a test sentence. ".repeat(50);

        let input = CreateContent {
            title: "Long Post".to_string(),
            slug: None,
            content_type: ContentType::Post,
            body_markdown: long_content,
            excerpt: None, // Should auto-generate
            featured_image: None,
            status: ContentStatus::Draft,
            scheduled_at: None,
            tags: vec![],
            metadata: None,
        };

        let content_id = content::create_content(&db, input, None, 100).unwrap();

        let post = content::get_content_by_id(&db, content_id)
            .unwrap()
            .expect("Content should exist");
        assert!(post.content.excerpt.is_some());
        // Excerpt should be truncated
        assert!(post.content.excerpt.as_ref().unwrap().len() <= 103); // 100 + "..."
    }

    #[test]
    fn test_slug_validation() {
        let db = create_test_db();

        let input = CreateContent {
            title: "Test".to_string(),
            slug: Some("Invalid Slug!".to_string()), // Invalid characters
            content_type: ContentType::Post,
            body_markdown: "Content".to_string(),
            excerpt: None,
            featured_image: None,
            status: ContentStatus::Draft,
            scheduled_at: None,
            tags: vec![],
            metadata: None,
        };

        let result = content::create_content(&db, input, None, 200);
        assert!(result.is_err());
    }

    #[test]
    fn test_content_metadata_defaults() {
        let db = create_test_db();

        // Create content without any custom metadata
        let input = CreateContent {
            title: "Metadata Test".to_string(),
            slug: None,
            content_type: ContentType::Page,
            body_markdown: "Test content".to_string(),
            excerpt: None,
            featured_image: None,
            status: ContentStatus::Draft,
            scheduled_at: None,
            tags: vec![],
            metadata: None, // No metadata provided
        };

        let content_id = content::create_content(&db, input, None, 200).unwrap();

        let page = content::get_content_by_id(&db, content_id)
            .unwrap()
            .expect("Content should exist");

        // Metadata should have default values for custom code fields
        // use_custom_code defaults to "none" for consistent handling
        assert_eq!(page.content.metadata["use_custom_code"], "none");
        assert_eq!(page.content.metadata["custom_html"], "");
        assert_eq!(page.content.metadata["custom_css"], "");
        assert_eq!(page.content.metadata["custom_js"], "");
    }

    #[test]
    fn test_content_with_custom_metadata() {
        let db = create_test_db();

        let custom_metadata = serde_json::json!({
            "use_custom_code": "only",
            "custom_html": "<div>Hello</div>",
            "custom_css": "body { color: red; }",
            "meta_title": "Custom Title"
        });

        let input = CreateContent {
            title: "Custom Page".to_string(),
            slug: None,
            content_type: ContentType::Page,
            body_markdown: "".to_string(),
            excerpt: None,
            featured_image: None,
            status: ContentStatus::Published,
            scheduled_at: None,
            tags: vec![],
            metadata: Some(custom_metadata),
        };

        let content_id = content::create_content(&db, input, None, 200).unwrap();

        let page = content::get_content_by_id(&db, content_id)
            .unwrap()
            .expect("Content should exist");

        assert_eq!(page.content.metadata["use_custom_code"], "only");
        assert_eq!(page.content.metadata["custom_html"], "<div>Hello</div>");
        assert_eq!(page.content.metadata["meta_title"], "Custom Title");
    }
}

mod settings_integration_tests {
    use super::*;

    #[test]
    fn test_get_setting_not_found() {
        let db = create_test_db();
        let result = settings::get_setting(&db, "nonexistent_key").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_set_and_get_setting() {
        let db = create_test_db();

        settings::set_setting(&db, "test_key", "test_value").unwrap();

        let value = settings::get_setting(&db, "test_key")
            .unwrap()
            .expect("Setting should exist");
        assert_eq!(value, "test_value");
    }

    #[test]
    fn test_update_setting() {
        let db = create_test_db();

        settings::set_setting(&db, "update_key", "original").unwrap();
        settings::set_setting(&db, "update_key", "updated").unwrap();

        let value = settings::get_setting(&db, "update_key")
            .unwrap()
            .expect("Setting should exist");
        assert_eq!(value, "updated");
    }

    #[test]
    fn test_delete_setting() {
        let db = create_test_db();

        settings::set_setting(&db, "delete_key", "value").unwrap();
        settings::delete_setting(&db, "delete_key").unwrap();

        let result = settings::get_setting(&db, "delete_key").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_settings_by_prefix() {
        let db = create_test_db();

        settings::set_setting(&db, "prefix_one", "value1").unwrap();
        settings::set_setting(&db, "prefix_two", "value2").unwrap();
        settings::set_setting(&db, "other_key", "value3").unwrap();

        let prefix_settings = settings::get_settings_by_prefix(&db, "prefix_").unwrap();
        assert_eq!(prefix_settings.len(), 2);
    }

    #[test]
    fn test_homepage_settings_defaults() {
        let db = create_test_db();

        let homepage = settings::get_homepage_settings(&db).unwrap();

        assert!(homepage.title.is_empty());
        assert!(homepage.subtitle.is_empty());
        assert!(homepage.show_pages); // Default true
        assert!(homepage.show_posts); // Default true
        assert!(homepage.custom_content.is_empty());
    }

    #[test]
    fn test_save_and_get_homepage_settings() {
        let db = create_test_db();

        let new_settings = settings::HomepageSettings {
            title: "Welcome".to_string(),
            subtitle: "My Blog".to_string(),
            show_pages: false,
            show_posts: true,
            custom_content: "<p>Hello World</p>".to_string(),
        };

        settings::save_homepage_settings(&db, &new_settings).unwrap();

        let loaded = settings::get_homepage_settings(&db).unwrap();

        assert_eq!(loaded.title, "Welcome");
        assert_eq!(loaded.subtitle, "My Blog");
        assert!(!loaded.show_pages);
        assert!(loaded.show_posts);
        assert_eq!(loaded.custom_content, "<p>Hello World</p>");
    }
}

mod search_integration_tests {
    use super::*;
    use pebble_cms::models::CreateContent;

    #[test]
    fn test_search_empty_results() {
        let db = create_test_db();

        let results = search::search_content(&db, "nonexistent", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_finds_published_content() {
        let db = create_test_db();

        // Create published post
        let input = CreateContent {
            title: "Rust Programming Guide".to_string(),
            slug: None,
            content_type: ContentType::Post,
            body_markdown: "Learn Rust programming language basics".to_string(),
            excerpt: None,
            featured_image: None,
            status: ContentStatus::Published,
            scheduled_at: None,
            tags: vec![],
            metadata: None,
        };
        content::create_content(&db, input, None, 200).unwrap();

        // Rebuild FTS index
        search::rebuild_fts_index(&db).unwrap();

        let results = search::search_content(&db, "rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Programming Guide");
    }

    #[test]
    fn test_search_excludes_draft_content() {
        let db = create_test_db();

        // Create draft post
        let input = CreateContent {
            title: "Draft About Python".to_string(),
            slug: None,
            content_type: ContentType::Post,
            body_markdown: "This is a draft about Python".to_string(),
            excerpt: None,
            featured_image: None,
            status: ContentStatus::Draft,
            scheduled_at: None,
            tags: vec![],
            metadata: None,
        };
        content::create_content(&db, input, None, 200).unwrap();

        search::rebuild_fts_index(&db).unwrap();

        let results = search::search_content(&db, "python", 10).unwrap();
        assert!(results.is_empty()); // Drafts should not appear
    }

    #[test]
    fn test_rebuild_fts_index() {
        let db = create_test_db();

        // Create multiple posts
        for i in 1..=3 {
            let input = CreateContent {
                title: format!("Post Number {}", i),
                slug: None,
                content_type: ContentType::Post,
                body_markdown: format!("Content for post {}", i),
                excerpt: None,
                featured_image: None,
                status: ContentStatus::Published,
                scheduled_at: None,
                tags: vec![],
                metadata: None,
            };
            content::create_content(&db, input, None, 200).unwrap();
        }

        let indexed = search::rebuild_fts_index(&db).unwrap();
        assert_eq!(indexed, 3);
    }

    #[test]
    fn test_search_multiple_terms() {
        let db = create_test_db();

        let input = CreateContent {
            title: "Web Development with JavaScript".to_string(),
            slug: None,
            content_type: ContentType::Post,
            body_markdown: "Learn web development using JavaScript and modern frameworks"
                .to_string(),
            excerpt: None,
            featured_image: None,
            status: ContentStatus::Published,
            scheduled_at: None,
            tags: vec![],
            metadata: None,
        };
        content::create_content(&db, input, None, 200).unwrap();

        search::rebuild_fts_index(&db).unwrap();

        // Search with multiple terms (OR logic)
        let results = search::search_content(&db, "javascript web", 10).unwrap();
        assert!(!results.is_empty());
    }
}

mod database_integration_tests {
    use super::*;

    #[test]
    fn test_run_analyze() {
        let db = create_test_db();
        // ANALYZE should succeed on empty database
        let result = database::run_analyze(&db);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_integrity_check() {
        let db = create_test_db();

        let results = database::run_integrity_check(&db).unwrap();

        // Should return "ok" for a healthy database
        assert!(!results.is_empty());
        assert_eq!(results[0], "ok");
    }

    #[test]
    fn test_run_integrity_check_with_data() {
        let db = create_test_db();

        // Add some data
        auth::create_user(
            &db,
            "testuser",
            "test@example.com",
            TEST_PASSWORD,
            UserRole::Admin,
        )
        .unwrap();

        let results = database::run_integrity_check(&db).unwrap();
        assert_eq!(results[0], "ok");
    }
}
