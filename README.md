<p align="center">
  <strong>Pebble</strong><br>
  A CMS that fits in your pocket.
</p>

<p align="center">
  <em>One binary. One database file. No runtime dependencies. Just your content.</em>
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> &bull;
  <a href="docs/usage.md">Full Documentation</a> &bull;
  <a href="#why-pebble">Why Pebble</a> &bull;
  <a href="docs/roadmap.md">Roadmap</a>
</p>

---

## TL;DR

Pebble is a lightweight CMS built in Rust. It compiles to a single binary with no external dependencies. Your entire site -- content, media, configuration -- lives in one SQLite file and a config.

```bash
pebble init my-blog && cd my-blog && pebble serve
```

That's it. Open `http://localhost:3000/admin`, create your account, start writing. When you're ready for production, run `pebble deploy`. When you want static hosting, run `pebble build`. Import from WordPress or Ghost. Export to Hugo or Zola. Back up everything with one command. No Docker required. No database server. No Node.js. No PHP. Just Pebble.

---

## Why Pebble

Most CMS platforms ask you to manage a database server, a runtime, a package manager, a cache layer, and a deployment pipeline before you write your first post. Pebble asks you to download one file.

**Zero infrastructure.** SQLite is embedded. The web server is embedded. Templates, CSS, and JavaScript are compiled into the binary. There is nothing to install, configure, or maintain besides Pebble itself.

**Two deployment modes.** Run `pebble serve` for a dynamic site with a full admin panel, or run `pebble build` to generate a static site you can drop onto any hosting provider. Switch between them whenever you want -- your content stays the same.

**Import and export freely.** Bring your WordPress or Ghost site with one command. Export to Hugo or Zola at any time. Your content is never locked in.

**Opinionated defaults, full control.** Fifteen built-in themes. Automatic image optimization with responsive WebP variants. Syntax-highlighted code blocks. RSS feeds. Sitemaps. Analytics. Audit logging. Content versioning. All included, all configurable, all without plugins.

---

## Quick Start

### Install

```bash
# Build from source
cargo install --path .

# Or build with optimizations
cargo build --release
# Binary is at ./target/release/pebble
```

### Create a Site

```bash
pebble init my-blog
cd my-blog
```

This creates a `pebble.toml` config file and initializes your SQLite database.

### Start Writing

```bash
pebble serve
```

Open [http://localhost:3000/admin](http://localhost:3000/admin). On first visit, you'll create your admin account. Then you're ready to write.

### Go to Production

```bash
pebble deploy
```

Binds to `0.0.0.0:8080` by default. The admin panel is disabled in production mode -- manage content in development, serve it in production.

### Generate a Static Site

```bash
pebble build --output ./public --base-url https://example.com
```

Produces a complete static site with HTML pages, RSS/JSON feeds, a sitemap, a search index, and all your media files. Deploy it to GitHub Pages, Netlify, Vercel, Cloudflare Pages, S3, or any web server.

---

## Features

### Content

- **Posts, Pages, and Snippets** -- three content types covering blogs, static pages, and reusable content blocks
- **Markdown with extras** -- tables, footnotes, strikethrough, task lists, and fenced code blocks with syntax highlighting for 17+ languages
- **Content series** -- group posts into ordered sequences with automatic previous/next navigation
- **Shortcodes** -- embed images, video, audio, and galleries directly in Markdown
- **Scheduled publishing** -- set a future publish date; Pebble publishes automatically
- **Content versioning** -- every edit creates a version snapshot you can view, compare, or restore
- **Draft previews** -- share unpublished content via signed, time-limited preview URLs
- **Bulk operations** -- publish, unpublish, archive, or delete multiple posts at once
- **Full-text search** -- built-in search powered by SQLite FTS5

### Media

- **Automatic image optimization** -- uploads are converted to WebP with responsive variants at 400w, 800w, 1200w, and 1600w
- **Responsive picture elements** -- shortcodes generate `<picture>` with `srcset` for automatic size selection
- **Drag-and-drop upload** -- drop images directly into the Markdown editor
- **Clipboard paste** -- paste screenshots and copied images into the editor
- **Supported formats** -- JPEG, PNG, GIF, WebP, SVG, MP4, WebM, MP3, OGG, PDF

### Themes

Fifteen built-in themes, all supporting light and dark mode:

| | | | |
|---|---|---|---|
| default | minimal | magazine | brutalist |
| neon | serif | ocean | midnight |
| botanical | monochrome | coral | terminal |
| nordic | sunset | typewriter | |

Every theme can be customized with your own colors, fonts, and spacing via `pebble.toml`. No CSS editing required.

### Admin Panel

- **Rich Markdown editor** with toolbar, keyboard shortcuts (Ctrl+B/I/K/S), and live preview
- **Auto-save drafts** to local storage -- never lose work
- **SEO metadata** -- custom meta titles, descriptions, and canonical URLs per page
- **Tag management** with autocomplete
- **Media library** with upload, browse, and delete
- **User management** with three roles: Admin, Author, Viewer
- **Database dashboard** -- view stats, run vacuum and analyze operations
- **Settings panel** -- configure site, theme, homepage layout, and content options from the browser

### Analytics

Built-in, privacy-respecting analytics. No third-party scripts. No cookies.

- **Privacy by default** -- IPs are anonymized, sessions are hashed, DNT is respected
- **Dashboard** -- pageviews, unique sessions, top pages, referrers, devices, browsers, and countries
- **Real-time view** -- active sessions and recent pageviews
- **Per-content stats** -- see how individual posts perform over time
- **Export** -- download analytics data as JSON or CSV

### API

A read-only JSON API at `/api/v1/` for headless CMS use cases, mobile apps, or integrations.

- Token-authenticated with `Bearer` header
- Endpoints for posts, pages, tags, series, media, and site info
- Paginated responses with consistent JSON envelope

### Webhooks

HTTP callbacks on content events. Trigger CI/CD rebuilds, Slack notifications, or any integration.

- **Events**: `content.published`, `content.updated`, `content.deleted`, `media.uploaded`, `media.deleted`
- **HMAC-SHA256 signing** for payload verification
- **Automatic retries** with exponential backoff
- **Delivery log** viewable in the admin panel

### Security

- **Argon2 password hashing**
- **Rate limiting** on all write endpoints (login, content, uploads, settings)
- **CSRF protection** on all admin forms
- **Content Security Policy** headers on every response
- **HttpOnly, Secure, SameSite=Strict** session cookies
- **Audit logging** -- every admin action is recorded with user, timestamp, and details
- **SVG sanitization** -- uploaded SVGs are checked for script injection
- **Path traversal protection** in backup restore

### Import & Export

| From | To |
|------|-----|
| WordPress (WXR XML) | Pebble (native Markdown) |
| Ghost (JSON export) | Hugo (TOML frontmatter) |
| Pebble export directory | Zola (TOML frontmatter) |

### Backups

- **Manual**: `pebble backup create` / `pebble backup restore`
- **Automatic**: configure scheduled backups with retention in `pebble.toml`
- **Format**: ZIP archive containing the database, all media files, and a manifest

### Multi-Site Registry

Manage multiple Pebble sites from one machine. Each site gets its own database, config, and port.

```bash
pebble registry init blog --title "My Blog"
pebble registry init docs --title "Documentation"
pebble registry serve blog    # auto-assigns port 3001
pebble registry serve docs    # auto-assigns port 3002
pebble registry list
pebble registry stop-all
```

---

## Commands

| Command | What it does |
|---------|-------------|
| `pebble init [path]` | Create a new site |
| `pebble serve` | Start development server (localhost:3000) |
| `pebble deploy` | Start production server (0.0.0.0:8080) |
| `pebble build` | Generate a static site |
| `pebble export` | Export content as Markdown (Pebble, Hugo, or Zola format) |
| `pebble import [path]` | Import from a Pebble export directory |
| `pebble import-wp <file>` | Import from WordPress WXR export |
| `pebble import-ghost <file>` | Import from Ghost JSON export |
| `pebble backup create` | Create a backup ZIP |
| `pebble backup restore <file>` | Restore from a backup ZIP |
| `pebble backup list` | List available backups |
| `pebble user add` | Create a user account |
| `pebble user list` | List all users |
| `pebble user passwd <name>` | Change a user's password |
| `pebble user remove <name>` | Delete a user |
| `pebble migrate` | Run database migrations |
| `pebble rerender` | Re-render all content HTML from Markdown |
| `pebble config list` | View global configuration |
| `pebble registry init <name>` | Create a registry-managed site |
| `pebble registry serve <name>` | Start a registry site |
| `pebble registry list` | Show all registry sites and status |
| `pebble registry stop-all` | Stop all running registry sites |

Run any command with `--help` for full options.

---

## Configuration

Each site is configured with a `pebble.toml` at the site root:

```toml
[site]
title = "My Site"
description = "A personal blog"
url = "https://example.com"
language = "en"

[server]
host = "127.0.0.1"
port = 3000

[database]
path = "./data/pebble.db"

[content]
posts_per_page = 10
excerpt_length = 200
auto_excerpt = true
version_retention = 50      # versions per content item (0 = unlimited)

[media]
upload_dir = "./data/media"
max_upload_size = "10MB"

[theme]
name = "default"

[theme.custom]              # optional overrides
primary_color = "#e63946"
font_family = "Georgia, serif"

[auth]
session_lifetime = "7d"

[api]
enabled = false             # set true to enable /api/v1/ endpoints

[backup]
auto_enabled = false        # set true for scheduled backups
interval_hours = 24
retention_count = 7
directory = "./backups"
```

See [docs/usage.md](docs/usage.md) for the complete configuration reference with all available options.

---

## Architecture

```
pebble (single binary)
  |
  +-- Axum web server
  |     +-- Public routes (posts, pages, tags, feeds, search, sitemap)
  |     +-- Admin routes (dashboard, editor, media, settings, users)
  |     +-- API routes (/api/v1/*)
  |     +-- HTMX routes (live preview, search, autocomplete)
  |
  +-- SQLite database (WAL mode, connection pooling, FTS5)
  |     +-- Content, users, sessions, tags, series, media metadata
  |     +-- Analytics, audit logs, versions, API tokens, webhooks
  |
  +-- Embedded assets
        +-- Tera HTML templates
        +-- CSS bundles (public + admin)
        +-- JavaScript (theme toggle, admin editor)
```

- **Language**: Rust
- **Web framework**: [Axum](https://github.com/tokio-rs/axum)
- **Database**: [SQLite](https://www.sqlite.org/) via rusqlite with r2d2 pooling
- **Templates**: [Tera](https://keats.github.io/tera/)
- **Markdown**: [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) with [syntect](https://github.com/trishume/syntect) highlighting
- **HTML sanitization**: [ammonia](https://github.com/rust-ammonia/ammonia)
- **Password hashing**: Argon2
- **Image processing**: [image](https://github.com/image-rs/image) crate

---

## Comparison

| | Pebble | WordPress | Ghost | Hugo |
|---|---|---|---|---|
| Dependencies | None | PHP, MySQL/MariaDB | Node.js, MySQL | None |
| Deployment | Single binary | LAMP stack | Node app | Build step + static host |
| Admin panel | Built-in | Built-in | Built-in | None |
| Dynamic + Static | Both | Dynamic only | Dynamic only | Static only |
| Database | SQLite (embedded) | MySQL/MariaDB | MySQL | None (files) |
| Image optimization | Automatic | Plugins | Built-in | External tools |
| Themes | 15 built-in | Install separately | Install separately | Install separately |
| Analytics | Built-in | Plugins | Built-in | External service |
| API | Built-in (token auth) | Built-in (REST) | Built-in (REST) | None |

---

## Documentation

- **[Full Usage Guide](docs/usage.md)** -- comprehensive reference for every feature, command, and configuration option
- **[Roadmap](docs/roadmap.md)** -- what's planned for v1.0 and beyond

---

## License

[MIT](LICENSE)
