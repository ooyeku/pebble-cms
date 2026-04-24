use crate::models::{ContentStatus, ContentType};
use crate::services::{content, settings, tags};
use crate::web::AppState;
use crate::Config;
use anyhow::Result;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tera::Context;

pub async fn run(config_path: &Path, output_dir: &Path, base_url: Option<String>) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = crate::Database::open(&config.database.path)?;

    let site_url = base_url.unwrap_or_else(|| config.site.url.clone());

    let state = Arc::new(AppState::new(
        config.clone(),
        config_path.to_path_buf(),
        db.clone(),
        true,
    )?);

    fs::create_dir_all(output_dir)?;

    tracing::info!("Building static site to {}", output_dir.display());

    build_index(&state, output_dir, &site_url)?;
    build_posts(&state, output_dir)?;
    build_pages(&state, output_dir)?;
    build_tags(&state, output_dir)?;
    build_search(&state, output_dir)?;
    build_feeds(&state, output_dir, &site_url)?;
    copy_media(&config, output_dir)?;

    tracing::info!("Static site build complete");
    Ok(())
}

fn make_context(state: &AppState) -> Context {
    let config = state.config();
    let mut ctx = Context::new();
    ctx.insert("site", &config.site);
    ctx.insert("theme", &config.theme);
    ctx.insert("homepage_config", &config.homepage);
    ctx.insert("production_mode", &true);
    ctx.insert("static_mode", &true);
    ctx.insert("user", &None::<()>);
    if config.theme.custom.has_customizations() {
        ctx.insert("theme_custom_css", &config.theme.custom.to_css_variables());
    }
    ctx
}

fn build_index(state: &AppState, output_dir: &Path, _site_url: &str) -> Result<()> {
    let posts_per_page = state.config().content.posts_per_page;
    let total = content::count_content(
        &state.db,
        Some(ContentType::Post),
        Some(ContentStatus::Published),
    )?;
    let total_pages = ((total as usize) + posts_per_page - 1) / posts_per_page;
    let homepage_settings = settings::get_homepage_settings(&state.db).unwrap_or_default();
    let pages = content::list_published_content(&state.db, ContentType::Page, 100, 0)?;

    for page_num in 1..=total_pages.max(1) {
        let offset = (page_num - 1) * posts_per_page;
        let posts =
            content::list_published_content(&state.db, ContentType::Post, posts_per_page, offset)?;

        let mut ctx = make_context(state);
        ctx.insert("posts", &posts);
        ctx.insert("pages", &pages);
        ctx.insert("homepage", &homepage_settings);
        ctx.insert("page", &page_num);
        ctx.insert("total_pages", &total_pages);
        ctx.insert("has_prev", &(page_num > 1));
        ctx.insert("has_next", &(page_num < total_pages));
        ctx.insert("prev_page", &(page_num - 1));
        ctx.insert("next_page", &(page_num + 1));

        let html = state.templates.render("public/index.html", &ctx)?;

        if page_num == 1 {
            fs::write(output_dir.join("index.html"), &html)?;
            fs::create_dir_all(output_dir.join("posts"))?;
            fs::write(output_dir.join("posts").join("index.html"), &html)?;
        }

        if total_pages > 1 {
            let page_dir = output_dir
                .join("posts")
                .join("page")
                .join(page_num.to_string());
            fs::create_dir_all(&page_dir)?;
            fs::write(page_dir.join("index.html"), &html)?;
        }
    }

    tracing::info!("Built index with {} page(s)", total_pages.max(1));
    Ok(())
}

fn build_posts(state: &AppState, output_dir: &Path) -> Result<()> {
    let posts = content::list_all_published_content(&state.db, ContentType::Post)?;
    let posts_dir = output_dir.join("posts");
    fs::create_dir_all(&posts_dir)?;

    for post in &posts {
        let mut ctx = make_context(state);
        ctx.insert("content", &post);

        let html = state.templates.render("public/post.html", &ctx)?;

        let post_dir = posts_dir.join(&post.content.slug);
        fs::create_dir_all(&post_dir)?;
        fs::write(post_dir.join("index.html"), html)?;
    }

    tracing::info!("Built {} posts", posts.len());
    Ok(())
}

fn build_pages(state: &AppState, output_dir: &Path) -> Result<()> {
    let pages = content::list_all_published_content(&state.db, ContentType::Page)?;

    for page in &pages {
        let mut ctx = make_context(state);
        ctx.insert("content", &page);

        let html = state.templates.render("public/page.html", &ctx)?;

        let page_dir = output_dir.join("pages").join(&page.content.slug);
        fs::create_dir_all(&page_dir)?;
        fs::write(page_dir.join("index.html"), html)?;
    }

    tracing::info!("Built {} pages", pages.len());
    Ok(())
}

fn build_tags(state: &AppState, output_dir: &Path) -> Result<()> {
    let tags_dir = output_dir.join("tags");
    fs::create_dir_all(&tags_dir)?;

    let all_tags = tags::list_published_tags_with_counts(&state.db)?;

    let mut ctx = make_context(state);
    ctx.insert("tags", &all_tags);
    let html = state.templates.render("public/tags.html", &ctx)?;
    fs::write(tags_dir.join("index.html"), html)?;

    for tag in &all_tags {
        let posts = tags::get_posts_by_tag(&state.db, &tag.tag.slug)?;
        let mut ctx = make_context(state);
        ctx.insert("tag", tag);
        ctx.insert("posts", &posts);

        let html = state.templates.render("public/tag.html", &ctx)?;

        let tag_dir = tags_dir.join(&tag.tag.slug);
        fs::create_dir_all(&tag_dir)?;
        fs::write(tag_dir.join("index.html"), html)?;
    }

    tracing::info!("Built {} tag pages", all_tags.len());
    Ok(())
}

fn build_search(state: &AppState, output_dir: &Path) -> Result<()> {
    let search_dir = output_dir.join("search");
    fs::create_dir_all(&search_dir)?;

    let posts = content::list_all_published_content(&state.db, ContentType::Post)?;

    let search_index: Vec<serde_json::Value> = posts
        .iter()
        .map(|post| {
            serde_json::json!({
                "slug": post.content.slug,
                "title": post.content.title,
                "excerpt": post.content.excerpt,
                "body": post.content.body_markdown,
            })
        })
        .collect();

    fs::write(
        search_dir.join("index.json"),
        serde_json::to_string(&search_index)?,
    )?;

    let search_html = generate_static_search_page(state)?;
    fs::write(search_dir.join("index.html"), search_html)?;

    tracing::info!("Built search page with {} indexed posts", posts.len());
    Ok(())
}

fn generate_static_search_page(state: &AppState) -> Result<String> {
    let mut ctx = make_context(state);
    ctx.insert("query", "");
    ctx.insert("results", &Vec::<()>::new());

    let template_html = state.templates.render("public/search.html", &ctx)?;

    let search_script = r#"
<script>
(function() {
    let searchIndex = null;
    const form = document.querySelector('#search-form');
    if (!form) return;
    const input = form.querySelector('input[name="q"]');
    const resultsSection = document.querySelector('#search-results');
    if (!input || !resultsSection) return;

    fetch('/search/index.json')
        .then(r => r.json())
        .then(data => { searchIndex = data; })
        .catch(console.error);

    form.addEventListener('submit', function(e) {
        e.preventDefault();
        performSearch(input.value.trim().toLowerCase());
    });

    input.addEventListener('input', function() {
        if (this.value.length > 2) {
            performSearch(this.value.trim().toLowerCase());
        } else if (this.value.length === 0) {
            resultsSection.innerHTML = '';
        }
    });

    function performSearch(query) {
        if (!searchIndex || !query) {
            resultsSection.innerHTML = '';
            return;
        }

        const results = searchIndex.filter(post =>
            post.title.toLowerCase().includes(query) ||
            (post.excerpt && post.excerpt.toLowerCase().includes(query)) ||
            post.body.toLowerCase().includes(query)
        );

        if (results.length === 0) {
            resultsSection.innerHTML = '<div class="results-header" role="status"><span class="results-count">0 results for </span><span class="results-query">"' + escapeHtml(query) + '"</span></div><div class="empty-state"><p>Try a different search term.</p></div>';
            return;
        }

        let html = '<div class="results-header" role="status"><span class="results-count">' + results.length + ' result' + (results.length !== 1 ? 's' : '') + ' for </span><span class="results-query">"' + escapeHtml(query) + '"</span></div><div class="results-list">';
        results.forEach(post => {
            html += '<article class="search-result search-result--post"><a href="/posts/' + encodeURIComponent(post.slug) + '" class="result-link"><span class="result-type">post</span><h2 class="result-title">' + escapeHtml(post.title) + '</h2>';
            if (post.excerpt) {
                html += '<p class="result-excerpt">' + escapeHtml(post.excerpt) + '</p>';
            }
            html += '</a></article>';
        });
        html += '</div>';
        resultsSection.innerHTML = html;
    }

    function escapeHtml(value) {
        return String(value).replace(/[&<>"']/g, function(ch) {
            return ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[ch];
        });
    }
})();
</script>
"#;

    let html = template_html.replace("</body>", &format!("{}</body>", search_script));

    Ok(html)
}

fn build_feeds(state: &AppState, output_dir: &Path, site_url: &str) -> Result<()> {
    let posts = content::list_published_content(&state.db, ContentType::Post, 20, 0)?;
    let config = state.config();

    let rss = generate_rss(&config.site, site_url, &posts);
    fs::write(output_dir.join("feed.xml"), rss)?;

    let json_feed = generate_json_feed(&config.site, site_url, &posts);
    fs::write(output_dir.join("feed.json"), json_feed)?;

    let sitemap = generate_sitemap(state, site_url)?;
    fs::write(output_dir.join("sitemap.xml"), sitemap)?;

    tracing::info!("Built RSS, JSON Feed, and sitemap");
    Ok(())
}

fn generate_rss(
    site: &crate::config::SiteConfig,
    site_url: &str,
    posts: &[crate::models::ContentWithTags],
) -> String {
    let mut items = String::new();
    for post in posts {
        let pub_date = post
            .content
            .published_at
            .as_ref()
            .unwrap_or(&post.content.created_at);
        let excerpt = post.content.excerpt.as_deref().unwrap_or("");
        items.push_str(&format!(
            r#"<item>
<title>{}</title>
<link>{}/posts/{}</link>
<guid>{}/posts/{}</guid>
<pubDate>{}</pubDate>
<description><![CDATA[{}]]></description>
</item>
"#,
            xml_escape(&post.content.title),
            site_url,
            post.content.slug,
            site_url,
            post.content.slug,
            pub_date,
            cdata_escape(excerpt)
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
<channel>
<title>{}</title>
<link>{}</link>
<description>{}</description>
<language>{}</language>
<atom:link href="{}/feed.xml" rel="self" type="application/rss+xml"/>
{}
</channel>
</rss>"#,
        xml_escape(&site.title),
        site_url,
        xml_escape(&site.description),
        site.language,
        site_url,
        items
    )
}

fn generate_json_feed(
    site: &crate::config::SiteConfig,
    site_url: &str,
    posts: &[crate::models::ContentWithTags],
) -> String {
    let items: Vec<serde_json::Value> = posts
        .iter()
        .map(|post| {
            serde_json::json!({
                "id": format!("{}/posts/{}", site_url, post.content.slug),
                "url": format!("{}/posts/{}", site_url, post.content.slug),
                "title": post.content.title,
                "content_html": post.content.body_html,
                "summary": post.content.excerpt,
                "date_published": post.content.published_at.as_ref().unwrap_or(&post.content.created_at),
                "tags": post.tags.iter().map(|t| &t.name).collect::<Vec<_>>()
            })
        })
        .collect();

    serde_json::json!({
        "version": "https://jsonfeed.org/version/1.1",
        "title": site.title,
        "home_page_url": site_url,
        "feed_url": format!("{}/feed.json", site_url),
        "description": site.description,
        "language": site.language,
        "items": items
    })
    .to_string()
}

fn generate_sitemap(state: &AppState, site_url: &str) -> Result<String> {
    let mut urls = String::new();

    urls.push_str(&format!(
        "<url><loc>{}</loc><changefreq>daily</changefreq><priority>1.0</priority></url>\n",
        site_url
    ));

    let posts = content::list_all_published_content(&state.db, ContentType::Post)?;
    for post in posts {
        urls.push_str(&format!(
            "<url><loc>{}/posts/{}</loc><lastmod>{}</lastmod><changefreq>weekly</changefreq></url>\n",
            site_url,
            post.content.slug,
            post.content.updated_at.split('T').next().unwrap_or(&post.content.updated_at)
        ));
    }

    let pages = content::list_all_published_content(&state.db, ContentType::Page)?;
    for page in pages {
        urls.push_str(&format!(
            "<url><loc>{}/pages/{}</loc><lastmod>{}</lastmod><changefreq>monthly</changefreq></url>\n",
            site_url,
            page.content.slug,
            page.content
                .updated_at
                .split('T')
                .next()
                .unwrap_or(&page.content.updated_at)
        ));
    }

    let all_tags = tags::list_published_tags_with_counts(&state.db)?;
    urls.push_str(&format!(
        "<url><loc>{}/tags</loc><changefreq>weekly</changefreq></url>\n",
        site_url
    ));
    for tag in all_tags {
        urls.push_str(&format!(
            "<url><loc>{}/tags/{}</loc><changefreq>weekly</changefreq></url>\n",
            site_url, tag.tag.slug
        ));
    }

    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
{}
</urlset>"#,
        urls
    ))
}

fn copy_media(config: &Config, output_dir: &Path) -> Result<()> {
    let media_src = Path::new(&config.media.upload_dir);
    if !media_src.exists() {
        return Ok(());
    }

    let media_dest = output_dir.join("media");
    fs::create_dir_all(&media_dest)?;

    let mut count = 0;
    for entry in fs::read_dir(media_src)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name() {
                fs::copy(&path, media_dest.join(filename))?;
                count += 1;
            }
        }
    }

    tracing::info!("Copied {} media files", count);
    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn cdata_escape(s: &str) -> String {
    s.replace("]]>", "]]]]><![CDATA[>")
}

#[cfg(test)]
mod tests {
    use super::cdata_escape;

    #[test]
    fn cdata_escape_splits_cdata_terminators() {
        assert_eq!(
            cdata_escape("excerpt ]]> tail"),
            "excerpt ]]]]><![CDATA[> tail"
        );
    }
}
