# Pebble CMS -- Complete Usage Guide

Everything you need to set up, configure, and run a Pebble site. This guide covers every feature, command, and configuration option.

---

## Table of Contents

- [Getting Started](#getting-started)
  - [Installation](#installation)
  - [Creating Your First Site](#creating-your-first-site)
  - [First-Time Setup](#first-time-setup)
  - [Development vs. Production](#development-vs-production)
- [CLI Reference](#cli-reference)
  - [Global Options](#global-options)
  - [pebble init](#pebble-init)
  - [pebble serve](#pebble-serve)
  - [pebble deploy](#pebble-deploy)
  - [pebble build](#pebble-build)
  - [pebble export](#pebble-export)
  - [pebble import](#pebble-import)
  - [pebble import-wp](#pebble-import-wp)
  - [pebble import-ghost](#pebble-import-ghost)
  - [pebble backup](#pebble-backup)
  - [pebble migrate](#pebble-migrate)
  - [pebble rerender](#pebble-rerender)
  - [pebble user](#pebble-user)
  - [pebble config](#pebble-config)
  - [pebble registry](#pebble-registry)
- [Configuration Reference](#configuration-reference)
  - [Site](#site)
  - [Server](#server)
  - [Database](#database)
  - [Content](#content)
  - [Media](#media)
  - [Theme](#theme)
  - [Theme Customization](#theme-customization)
  - [Authentication](#authentication)
  - [Homepage](#homepage)
  - [Audit](#audit)
  - [API](#api)
  - [Backup](#backup)
- [Writing Content](#writing-content)
  - [Content Types](#content-types)
  - [Content Statuses](#content-statuses)
  - [The Markdown Editor](#the-markdown-editor)
  - [Markdown Features](#markdown-features)
  - [SEO Metadata](#seo-metadata)
  - [Custom Page Code](#custom-page-code)
- [Shortcodes](#shortcodes)
  - [image](#image)
  - [video](#video)
  - [audio](#audio)
  - [media](#media-shortcode)
  - [gallery](#gallery)
  - [snippet](#snippet)
- [Content Series](#content-series)
  - [Creating a Series](#creating-a-series)
  - [Public Series Pages](#public-series-pages)
  - [Series Navigation on Posts](#series-navigation-on-posts)
- [Snippets](#snippets)
  - [Creating Snippets](#creating-snippets)
  - [Embedding Snippets](#embedding-snippets)
- [Media Management](#media-management)
  - [Supported File Types](#supported-file-types)
  - [Uploading Media](#uploading-media)
  - [Image Optimization Pipeline](#image-optimization-pipeline)
  - [Responsive Images](#responsive-images)
- [Themes](#themes)
  - [Available Themes](#available-themes)
  - [Setting a Theme](#setting-a-theme)
  - [Customizing Colors and Typography](#customizing-colors-and-typography)
  - [Dark Mode](#dark-mode)
- [User Management](#user-management)
  - [Roles and Permissions](#roles-and-permissions)
  - [CLI User Commands](#cli-user-commands)
  - [Admin Panel User Management](#admin-panel-user-management)
- [Analytics](#analytics)
  - [How It Works](#how-analytics-works)
  - [Privacy](#analytics-privacy)
  - [Dashboard](#analytics-dashboard)
  - [Exporting Data](#exporting-analytics-data)
- [REST API](#rest-api)
  - [Enabling the API](#enabling-the-api)
  - [Authentication](#api-authentication)
  - [Managing Tokens](#managing-tokens)
  - [Endpoints](#api-endpoints)
  - [Response Format](#api-response-format)
- [Webhooks](#webhooks)
  - [Setting Up Webhooks](#setting-up-webhooks)
  - [Events](#webhook-events)
  - [Payload Signing](#payload-signing)
  - [Delivery and Retries](#delivery-and-retries)
  - [Delivery Log](#delivery-log)
- [Content Versioning](#content-versioning)
  - [How Versions Work](#how-versions-work)
  - [Viewing History](#viewing-history)
  - [Comparing Versions](#comparing-versions)
  - [Restoring a Version](#restoring-a-version)
- [Audit Logging](#audit-logging)
  - [What Is Logged](#what-is-logged)
  - [Viewing Audit Logs](#viewing-audit-logs)
  - [Filtering and Exporting](#filtering-and-exporting-audit-logs)
- [Backup and Restore](#backup-and-restore)
  - [Manual Backups](#manual-backups)
  - [Automatic Backups](#automatic-backups)
  - [Backup Retention](#backup-retention)
  - [Restoring from Backup](#restoring-from-backup)
- [Import and Export](#import-and-export)
  - [Importing from WordPress](#importing-from-wordpress)
  - [Importing from Ghost](#importing-from-ghost)
  - [Importing from Pebble Export](#importing-from-pebble-export)
  - [Exporting Content](#exporting-content)
- [Static Site Generation](#static-site-generation)
  - [Building a Static Site](#building-a-static-site)
  - [Output Structure](#output-structure)
  - [Deployment Options](#deployment-options)
- [Multi-Site Registry](#multi-site-registry)
  - [Overview](#registry-overview)
  - [Creating Sites](#creating-registry-sites)
  - [Starting and Stopping](#starting-and-stopping-sites)
  - [Site Configuration](#registry-site-configuration)
  - [Site Logs](#site-logs)
- [Feeds and Discovery](#feeds-and-discovery)
  - [RSS Feed](#rss-feed)
  - [JSON Feed](#json-feed)
  - [Tag RSS Feeds](#tag-rss-feeds)
  - [Sitemap](#sitemap)
  - [Health Check](#health-check)
  - [Draft Previews](#draft-previews)
- [Security](#security)
  - [Rate Limiting](#rate-limiting)
  - [Session Security](#session-security)
  - [Content Security Policy](#content-security-policy)
  - [CSRF Protection](#csrf-protection)
  - [SVG Sanitization](#svg-sanitization)
- [Performance](#performance)
  - [SQLite Tuning](#sqlite-tuning)
  - [Connection Pooling](#connection-pooling)
  - [Graceful Shutdown](#graceful-shutdown)
- [Global Configuration](#global-configuration)
  - [Config File Location](#config-file-location)
  - [Available Global Settings](#available-global-settings)
- [Environment Variables](#environment-variables)
- [Troubleshooting](#troubleshooting)

---

## Getting Started

### Installation

Pebble compiles to a single binary with no runtime dependencies.

```bash
# Clone the repository
git clone https://github.com/your-org/pebble-cms.git
cd pebble-cms

# Build and install
cargo install --path .
```

For an optimized production binary:

```bash
cargo build --release
# Binary is at ./target/release/pebble
```

The release build enables LTO, strips debug symbols, and produces a compact binary.

**Requirements**: Rust 1.75 or later.

### Creating Your First Site

```bash
pebble init my-blog
cd my-blog
pebble serve
```

This creates:
- `pebble.toml` -- your site configuration
- `data/pebble.db` -- the SQLite database
- `data/media/` -- the media upload directory

Open [http://localhost:3000/admin](http://localhost:3000/admin) to continue.

### First-Time Setup

When no users exist in the database, Pebble shows a setup page at `/admin`. Enter your desired username, email, and password to create the first admin account. All subsequent users are created through the admin panel or CLI.

### Development vs. Production

Pebble has two server modes:

| | `pebble serve` | `pebble deploy` |
|---|---|---|
| Default bind address | `127.0.0.1:3000` | `0.0.0.0:8080` |
| Admin panel | Enabled | Disabled (returns 404) |
| Database migrations | Runs automatically | Does not run |
| Scheduled publishing | Enabled (checks every 60s) | Disabled |
| HTMX endpoints | Enabled | Disabled |
| Auto-backup | If configured | If configured |

**Typical workflow**: Use `pebble serve` to write and manage content. Deploy with `pebble deploy` to serve your site to the public, or use `pebble build` to generate static files.

---

## CLI Reference

### Global Options

All commands accept:

| Flag | Description | Default |
|------|-------------|---------|
| `-c, --config <PATH>` | Path to configuration file | `pebble.toml` |

### pebble init

Create a new Pebble site.

```bash
pebble init                     # Current directory
pebble init my-blog             # New directory
pebble init --name "My Blog"    # Set site name
```

### pebble serve

Start the development server with the full admin panel.

```bash
pebble serve                            # 127.0.0.1:3000
pebble serve -H 0.0.0.0 -p 8080        # Custom host and port
```

| Flag | Description | Default |
|------|-------------|---------|
| `-H, --host <HOST>` | Bind address | `127.0.0.1` |
| `-p, --port <PORT>` | Port number | `3000` |

On startup, Pebble automatically runs database migrations, rebuilds the full-text search index, and starts the scheduled content publisher.

### pebble deploy

Start the production server. The admin panel is completely disabled.

```bash
pebble deploy                           # 0.0.0.0:8080
pebble deploy -H 127.0.0.1 -p 3000     # Custom host and port
```

| Flag | Description | Default |
|------|-------------|---------|
| `-H, --host <HOST>` | Bind address | `0.0.0.0` |
| `-p, --port <PORT>` | Port number | `8080` |

### pebble build

Generate a complete static site.

```bash
pebble build                                         # Output to ./dist
pebble build -o ./public                             # Custom output directory
pebble build --base-url https://example.com          # Set base URL for all links
```

| Flag | Description | Default |
|------|-------------|---------|
| `-o, --output <DIR>` | Output directory | `./dist` |
| `--base-url <URL>` | Base URL for links in the generated site | Site URL from config |

### pebble export

Export site content as Markdown files with frontmatter.

```bash
pebble export                                # Default: Pebble format to ./export
pebble export --format hugo -o ./hugo-site   # Hugo format
pebble export --format zola -o ./zola-site   # Zola format
pebble export --include-drafts               # Include non-published content
pebble export --include-media                # Copy media files to output
```

| Flag | Description | Default |
|------|-------------|---------|
| `-o, --output <DIR>` | Output directory | `./export` |
| `--format <FORMAT>` | Output format: `pebble`, `hugo`, or `zola` | `pebble` |
| `--include-drafts` | Include draft, scheduled, and archived content | Off |
| `--include-media` | Copy media files into the export | Off |

**Format details:**

- **pebble**: YAML frontmatter (`---`), posts in `posts/`, pages in `pages/`
- **hugo**: TOML frontmatter (`+++`), posts in `content/posts/`, pages in `content/`, media in `static/media/`
- **zola**: TOML frontmatter (`+++`) with `[taxonomies]` and `[extra]` sections, posts in `content/blog/`, media in `static/media/`

### pebble import

Import content from a Pebble export directory.

```bash
pebble import                    # Import from ./export
pebble import ./backup           # Import from a specific directory
pebble import --overwrite        # Replace existing content with matching slugs
```

### pebble import-wp

Import content from a WordPress WXR (XML) export file.

```bash
pebble import-wp wordpress-export.xml
pebble import-wp wordpress-export.xml --overwrite
```

Pebble parses WordPress posts and pages, converts HTML to Markdown, maps tags, and preserves publication status. Unsupported post types (attachments, nav menus, etc.) are skipped.

### pebble import-ghost

Import content from a Ghost JSON export file.

```bash
pebble import-ghost ghost-export.json
pebble import-ghost ghost-export.json --overwrite
```

Pebble extracts posts and pages from Ghost's export format, converts HTML or Mobiledoc content to Markdown, maps tags, and preserves publication status.

### pebble backup

Manage site backups.

```bash
pebble backup create                         # Save to ./backups
pebble backup create -o /mnt/external        # Custom backup directory
pebble backup list                           # List backups in ./backups
pebble backup list -d /mnt/external          # List backups in custom directory
pebble backup restore ./backups/pebble-backup-20250115_120000.zip
```

| Subcommand | Flags | Description |
|------------|-------|-------------|
| `create` | `-o, --output <DIR>` (default: `./backups`) | Create a timestamped backup ZIP |
| `list` | `-d, --dir <DIR>` (default: `./backups`) | List available backups with sizes |
| `restore` | `<file>` (required) | Restore database and media from a backup |

### pebble migrate

Run database migrations to apply schema updates after upgrading Pebble.

```bash
pebble migrate
```

Migrations are also run automatically when using `pebble serve`.

### pebble rerender

Re-render all content HTML from the stored Markdown. Useful after upgrading Pebble if the Markdown renderer has changed.

```bash
pebble rerender
```

### pebble user

Manage user accounts from the command line.

```bash
pebble user add --username alice --email alice@example.com --role admin
pebble user add --username bob --email bob@example.com --role author --password secret123
pebble user list
pebble user passwd alice           # Prompts for new password
pebble user remove alice
```

| Subcommand | Flags |
|------------|-------|
| `add` | `--username` (required), `--email` (required), `--role` (default: `author`), `--password` (optional; prompts if omitted) |
| `list` | None |
| `passwd` | `<username>` (required) |
| `remove` | `<username>` (required) |

### pebble config

Manage global Pebble configuration stored in `~/.pebble/config.toml`.

```bash
pebble config list                                 # Show all settings
pebble config get defaults.theme                   # Get a specific value
pebble config set defaults.theme minimal           # Set a value
pebble config remove defaults.theme                # Remove (reset to default)
pebble config path                                 # Show config file path
```

### pebble registry

Manage multiple Pebble sites from a central registry at `~/.pebble/registry/`.

```bash
pebble registry init mysite --title "My Site"      # Create a site
pebble registry list                               # List all sites with status
pebble registry serve mysite                       # Start in dev mode
pebble registry serve mysite -p 3005               # Start on specific port
pebble registry deploy mysite                      # Start in production mode
pebble registry status mysite                      # Check if running
pebble registry stop mysite                        # Stop a site
pebble registry stop-all                           # Stop all sites
pebble registry remove mysite                      # Remove from registry
pebble registry remove mysite --force              # Remove without confirmation
pebble registry path                               # Show registry directory
pebble registry path mysite                        # Show a site's directory
pebble registry rerender mysite                    # Re-render content
pebble registry config mysite                      # View site config
pebble registry config mysite get theme.name       # Get a config value
pebble registry config mysite set theme.name serif # Set a config value
pebble registry config mysite edit                 # Open config in $EDITOR
```

Ports are automatically assigned from the configured range (default: 3001-3100) if not specified.

---

## Configuration Reference

Pebble is configured through `pebble.toml` at the site root. Every section is documented below with all available fields and defaults.

### Site

```toml
[site]
title = "My Site"              # Displayed in headers, feeds, and metadata
description = "A personal blog" # Used in meta tags and feeds
url = "http://localhost:3000"  # Base URL (used for feeds, sitemap, canonical URLs)
language = "en"                # Language code (used in HTML lang attribute)
```

### Server

```toml
[server]
host = "127.0.0.1"            # Bind address
port = 3000                    # Port number
```

### Database

```toml
[database]
path = "./data/pebble.db"     # Path to SQLite database file
pool_size = 10                 # Connection pool size (default: 10)
```

### Content

```toml
[content]
posts_per_page = 10            # Posts per page in listings (1-100)
excerpt_length = 200           # Auto-excerpt character limit (1-10000)
auto_excerpt = true            # Generate excerpts from content automatically
version_retention = 50         # Max versions per content item (0 = unlimited)
```

### Media

```toml
[media]
upload_dir = "./data/media"    # Directory for uploaded files
max_upload_size = "10MB"       # Maximum file upload size
```

### Theme

```toml
[theme]
name = "default"               # One of 15 built-in themes (see Themes section)
```

### Theme Customization

Add a `[theme.custom]` section to override any theme's visual properties. All fields are optional.

```toml
[theme.custom]
# Colors
primary_color = "#007acc"           # Primary brand color
primary_color_hover = "#005a9e"     # Primary color hover state
accent_color = "#ff6b6b"           # Accent/highlight color
background_color = "#ffffff"        # Page background
background_secondary = "#f5f5f5"    # Secondary background (cards, code blocks)
text_color = "#333333"             # Main text color
text_muted = "#666666"             # Secondary/muted text
border_color = "#e0e0e0"           # Border color
link_color = "#007acc"             # Link color

# Typography
font_family = "system-ui, -apple-system, sans-serif"
heading_font_family = "Georgia, serif"
font_size = "16px"                  # Base font size
line_height = 1.6                   # Base line height

# Spacing
border_radius = "4px"              # Border radius for cards and buttons
```

These map to CSS custom properties and apply across the entire site without editing any CSS files.

### Authentication

```toml
[auth]
session_lifetime = "7d"        # How long sessions last (e.g., "7d", "24h", "1h")
```

### Homepage

```toml
[homepage]
show_hero = true                    # Show hero section
hero_layout = "centered"            # "centered", "split", or "minimal"
hero_height = "medium"              # "small", "medium", "large", or "full"
hero_text_align = "center"          # "left", "center", or "right"
hero_image = "/media/hero.jpg"      # Optional hero background image

show_posts = true                   # Show recent posts section
posts_layout = "grid"               # "grid" or "list"
posts_columns = 2                   # Grid columns (1-4)

show_pages = true                   # Show pages section
pages_layout = "grid"               # "grid" or "list"

sections_order = ["hero", "pages", "posts"]  # Order of homepage sections
```

### Audit

```toml
[audit]
enabled = true                 # Enable audit logging
retention_days = 90            # How long to keep audit records
log_auth_events = true         # Log login/logout events
log_content_views = false      # Log content view events
```

### API

```toml
[api]
enabled = false                # Enable the /api/v1/ JSON endpoints
default_page_size = 20         # Default items per page in API responses
max_page_size = 100            # Maximum items per page
```

### Backup

```toml
[backup]
auto_enabled = false           # Enable automatic scheduled backups
interval_hours = 24            # Hours between automatic backups
retention_count = 7            # Number of backups to keep (oldest are deleted)
directory = "./backups"        # Where to store backup files
```

---

## Writing Content

### Content Types

Pebble has three content types:

| Type | URL Pattern | Use Case |
|------|-------------|----------|
| **Post** | `/posts/{slug}` | Blog posts, articles, news -- displayed chronologically with pagination |
| **Page** | `/pages/{slug}` | Static pages like About, Contact, Projects -- not included in post listings |
| **Snippet** | Not directly accessible | Reusable content blocks embedded in posts/pages via the `[snippet]` shortcode |

### Content Statuses

| Status | Behavior |
|--------|----------|
| **Draft** | Not visible on the public site. Only accessible in the admin panel. |
| **Scheduled** | Not yet visible. Automatically published when the `scheduled_at` time is reached (checked every 60 seconds in `serve` mode). |
| **Published** | Visible on the public site, included in feeds and search. |
| **Archived** | Not visible on the public site. Preserved in the database for reference but returns 404 on direct access. |

### The Markdown Editor

The admin panel includes a Markdown editor with these features:

**Toolbar buttons:**

| Button | Action | Markdown Inserted |
|--------|--------|-------------------|
| **B** | Bold | `**text**` |
| *I* | Italic | `*text*` |
| Link | Insert link | `[text](url)` |
| H | Heading | `## ` prefix |
| `</>` | Inline code | `` `text` `` |
| Quote | Blockquote | `> ` prefix |
| Bullet | Unordered list | `- ` prefix |
| 1. | Ordered list | `1. ` prefix |
| Image | Insert image | `![alt](url)` |

**Keyboard shortcuts:**

| Shortcut | Action |
|----------|--------|
| Ctrl/Cmd + S | Save the current form |
| Ctrl/Cmd + Shift + P | Set status to Published and save |
| Ctrl/Cmd + B | Bold selected text |
| Ctrl/Cmd + I | Italicize selected text |
| Ctrl/Cmd + K | Wrap selected text as a link |

**Auto-save drafts:**

The editor saves your work to the browser's local storage every 2 seconds. If you accidentally navigate away, the editor offers to restore your unsaved draft on the next visit. Drafts are cleared after a successful save.

**Image upload in the editor:**

- **Drag and drop**: Drag an image file onto the editor textarea. It uploads automatically and inserts the Markdown image syntax.
- **Clipboard paste**: Copy an image (e.g., a screenshot) and paste with Ctrl/Cmd+V. The image uploads and is inserted at the cursor position.

A `![Uploading...]()` placeholder appears during upload and is replaced with the actual path on completion.

### Markdown Features

Pebble's Markdown renderer supports:

- **Tables** -- standard pipe syntax
- **Footnotes** -- `[^1]` references with definitions
- **Strikethrough** -- `~~deleted text~~`
- **Task lists** -- `- [x] done` / `- [ ] pending`
- **Custom heading IDs** -- `## My Heading {#custom-id}`
- **Auto-generated heading IDs** -- headings without custom IDs get slugified IDs for anchor links
- **Syntax-highlighted code blocks** -- fenced with language identifier

**Supported languages for syntax highlighting:**

Rust, Python, JavaScript, TypeScript, Go, C, C++, Java, HTML, CSS, JSON, YAML, TOML, SQL, Bash, Shell, Markdown.

### SEO Metadata

Each post and page supports optional SEO fields in the editor:

- **Meta title** -- overrides the default `<title>` tag
- **Meta description** -- sets the `<meta name="description">` tag
- **Canonical URL** -- sets the `<link rel="canonical">` tag

### Custom Page Code

Pages support optional custom HTML, CSS, and JavaScript. In the page editor, toggle "Use Custom Code" to enter raw code that renders instead of (or alongside) the Markdown content.

---

## Shortcodes

Shortcodes let you embed rich media in your Markdown content. They are processed during rendering and produce optimized HTML.

### image

```markdown
[image src="photo.jpg"]
[image src="photo.jpg" alt="A sunset" title="Summer 2024"]
[image src="photo.jpg" alt="Sunset" width="800" height="600" class="hero-image"]
```

| Attribute | Required | Description |
|-----------|----------|-------------|
| `src` | Yes | Filename of uploaded media |
| `alt` | No | Alternative text for accessibility |
| `title` | No | Title tooltip on hover |
| `width` | No | Display width |
| `height` | No | Display height |
| `class` | No | CSS class (default: `media-image`) |

Images render as responsive `<picture>` elements with WebP `srcset` variants when available.

The `[img]` shortcode is an alias for `[image]`.

### video

```markdown
[video src="demo.mp4"]
[video src="demo.mp4" autoplay muted loop]
[video src="demo.mp4" poster="thumbnail.jpg" width="1280" height="720"]
```

| Attribute | Required | Description |
|-----------|----------|-------------|
| `src` | Yes | Filename of uploaded video |
| `controls` | No | Show player controls (default: on) |
| `nocontrols` | No | Hide player controls |
| `autoplay` | No | Auto-play (browsers require `muted` for autoplay) |
| `muted` | No | Mute audio |
| `loop` | No | Loop playback |
| `poster` | No | Thumbnail image filename |
| `width` | No | Display width |
| `height` | No | Display height |
| `class` | No | CSS class (default: `media-video`) |

### audio

```markdown
[audio src="podcast.mp3"]
[audio src="music.ogg" loop]
```

| Attribute | Required | Description |
|-----------|----------|-------------|
| `src` | Yes | Filename of uploaded audio |
| `controls` | No | Show player controls (default: on) |
| `nocontrols` | No | Hide player controls |
| `autoplay` | No | Auto-play |
| `loop` | No | Loop playback |
| `class` | No | CSS class (default: `media-audio`) |

### media (shortcode)

Auto-detects the media type from the file extension and renders the appropriate element.

```markdown
[media src="photo.jpg"]        <!-- renders as image -->
[media src="demo.mp4"]         <!-- renders as video -->
[media src="podcast.mp3"]      <!-- renders as audio -->
[media src="document.pdf"]     <!-- renders as embedded PDF -->
[media src="archive.zip"]      <!-- renders as download link -->
```

Type detection:
- **Image**: jpg, jpeg, png, gif, webp, svg
- **Video**: mp4, webm
- **Audio**: mp3, ogg
- **PDF**: pdf
- **Other**: download link

### gallery

```markdown
[gallery src="img1.jpg,img2.jpg,img3.jpg"]
[gallery src="img1.jpg,img2.jpg,img3.jpg,img4.jpg" columns="4"]
```

| Attribute | Required | Description |
|-----------|----------|-------------|
| `src` | Yes | Comma-separated list of image filenames |
| `columns` | No | Grid columns (default: 3) |
| `class` | No | CSS class (default: `media-gallery`) |

Renders a CSS grid gallery with responsive WebP thumbnails.

### snippet

```markdown
[snippet slug="disclaimer"]
```

Embeds the rendered content of a snippet (see [Snippets](#snippets)).

---

## Content Series

Series let you group posts into ordered sequences -- ideal for tutorials, multi-part articles, or book chapters.

### Creating a Series

1. Go to **Series** in the admin sidebar
2. Click **New Series**
3. Enter a title, slug (auto-generated if blank), and description
4. Set the status to Draft or Published
5. Add posts from the dropdown
6. Drag to reorder posts within the series
7. Save

### Public Series Pages

Published series are accessible at `/series/{slug}`. The page shows the series title, description, and a numbered list of posts. Published posts link to their content; unpublished posts appear greyed out.

### Series Navigation on Posts

When a post belongs to a published series, a navigation bar appears below the post content with:
- The series name and a link back to the series page
- "Part X of Y" indicator
- Previous and next links within the series

---

## Snippets

Snippets are reusable content blocks managed in the admin panel and embedded in other content via shortcode.

### Creating Snippets

1. Go to **Snippets** in the admin sidebar
2. Click **New Snippet**
3. Enter a title and slug
4. Write the content in Markdown
5. Save

### Embedding Snippets

Use the snippet shortcode in any post or page:

```markdown
Here's our standard disclaimer:

[snippet slug="disclaimer"]

The rest of the post continues here.
```

The snippet's rendered Markdown replaces the shortcode at display time.

---

## Media Management

### Supported File Types

| Type | Formats | Size Limit |
|------|---------|------------|
| Images | JPEG, PNG, GIF, WebP, SVG | 50 MB |
| Video | MP4, WebM | 50 MB |
| Audio | MP3, OGG | 50 MB |
| Documents | PDF | 50 MB |

### Uploading Media

**Admin media library**: Go to **Media** in the admin sidebar. Use the upload form to select files.

**Editor drag-and-drop**: Drag image files directly onto the Markdown editor textarea.

**Editor clipboard paste**: Copy an image (screenshot, web image) and paste into the editor with Ctrl/Cmd+V.

### Image Optimization Pipeline

When you upload a JPEG, PNG, GIF, or WebP image, Pebble automatically:

1. Saves the original file
2. Creates a WebP conversion
3. Generates a thumbnail (`-thumb.webp`)
4. Creates responsive variants: `400w`, `800w`, `1200w`, and `1600w` WebP files

SVGs and non-image files are stored as-is.

### Responsive Images

When images are embedded via shortcodes, Pebble generates `<picture>` elements with `srcset`:

```html
<picture>
  <source srcset="/media/photo-400w.webp 400w,
                  /media/photo-800w.webp 800w,
                  /media/photo-1200w.webp 1200w,
                  /media/photo.webp 1600w"
          sizes="(max-width: 400px) 400px, (max-width: 800px) 800px,
                 (max-width: 1200px) 1200px, 1600px"
          type="image/webp">
  <img src="/media/photo.jpg" alt="description" loading="lazy">
</picture>
```

The browser automatically selects the best size for the viewer's screen and connection.

---

## Themes

### Available Themes

| Theme | Style |
|-------|-------|
| `default` | Clean, modern design with balanced spacing |
| `minimal` | Stripped-down, content-focused with minimal decoration |
| `magazine` | Multi-column, editorial layout |
| `brutalist` | Bold, unconventional aesthetics |
| `neon` | Dark theme with vibrant accent colors |
| `serif` | Classic, serif-focused typography |
| `ocean` | Cool, blue-toned palette |
| `midnight` | Deep dark theme with soft accents |
| `botanical` | Nature-inspired, earthy color palette |
| `monochrome` | Black and white, no color distractions |
| `coral` | Warm, coral-accented palette |
| `terminal` | Green-on-black monospace aesthetic |
| `nordic` | Clean, Scandinavian-inspired design |
| `sunset` | Warm gradient tones |
| `typewriter` | Classic typewriter-inspired monospace look |

### Setting a Theme

In `pebble.toml`:

```toml
[theme]
name = "minimal"
```

Or through the admin panel at **Settings**.

Or via the registry CLI:

```bash
pebble registry config mysite set theme.name serif
```

### Customizing Colors and Typography

Add a `[theme.custom]` section to override any theme's defaults. See the [Theme Customization](#theme-customization) section in the configuration reference for all available properties.

Example -- applying a custom color scheme to the minimal theme:

```toml
[theme]
name = "minimal"

[theme.custom]
primary_color = "#2d6a4f"
accent_color = "#52b788"
background_color = "#f8f9fa"
text_color = "#212529"
heading_font_family = "'Playfair Display', Georgia, serif"
```

### Dark Mode

All themes support both light and dark modes. Users can toggle between them using the theme toggle button.

Most themes default to light mode. Three themes default to dark mode: **neon**, **midnight**, and **terminal**. These use dark mode as their base and override styles for light mode.

---

## User Management

### Roles and Permissions

| Role | Content | Media | Settings | Users | Analytics | Audit |
|------|---------|-------|----------|-------|-----------|-------|
| **Admin** | Full access | Full access | Full access | Full access | View | View |
| **Author** | Create/edit own | Upload/delete | No access | No access | No access | No access |
| **Viewer** | Read-only | Read-only | No access | No access | No access | No access |

### CLI User Commands

```bash
# Create an admin user (prompts for password)
pebble user add --username alice --email alice@example.com --role admin

# Create an author with an inline password
pebble user add --username bob --email bob@example.com --role author --password secret123

# List all users
pebble user list

# Change a password (prompts interactively)
pebble user passwd alice

# Remove a user
pebble user remove alice
```

### Admin Panel User Management

Admins can create, edit, and delete users through **Users** in the admin sidebar. User roles can be changed and emails updated from the admin panel.

---

## Analytics

### How Analytics Works

Pebble includes built-in, server-side analytics. No client-side JavaScript is injected. Page views are recorded from server logs with privacy protections applied before storage.

Analytics tracks: page paths, referrer domains, device types (desktop/mobile/tablet), browser families, country codes, and response times.

### Analytics Privacy

- **IP anonymization**: IPv4 addresses have the last two octets zeroed. IPv6 keeps only the first three segments.
- **Session hashing**: Sessions are identified by a SHA-256 hash of `daily_salt + anonymized_ip + browser_family`. Only the first 16 characters of the hash are stored.
- **Daily salt rotation**: The hash salt changes daily, preventing long-term tracking.
- **DNT respected**: By default, requests with the `Do Not Track` header are not recorded.
- **No cookies**: Analytics does not set any cookies.
- **Excluded paths**: Admin, API, static assets, and health check paths are excluded automatically.

### Analytics Dashboard

Access analytics at **Analytics** in the admin sidebar. The dashboard shows:

- **Summary**: Total pageviews, unique sessions, percentage changes vs. prior period, average response time, error rate
- **Top pages**: Most-viewed paths
- **Top referrers**: Traffic sources
- **Devices and browsers**: Visitor breakdown
- **Countries**: Geographic distribution
- **Time series**: Pageviews over time
- **Real-time**: Active sessions (last 5 minutes) and recent pageviews (last 30 minutes)
- **Per-content stats**: Individual post performance, trends, and bounce rates

### Exporting Analytics Data

Export analytics as JSON or CSV from the dashboard, or via the API:

```
GET /api/analytics/export?days=30&format=json
GET /api/analytics/export?days=30&format=csv
```

---

## REST API

### Enabling the API

The API is disabled by default. Enable it in `pebble.toml`:

```toml
[api]
enabled = true
default_page_size = 20
max_page_size = 100
```

### API Authentication

All `/api/v1/` endpoints require a valid API token in the `Authorization` header:

```
Authorization: Bearer pb_abc123...
```

Tokens are prefixed with `pb_` and contain 32 bytes of cryptographic randomness. Only the SHA-256 hash of the token is stored -- the raw token is shown once at creation time and cannot be retrieved afterward.

### Managing Tokens

**Admin panel**: Go to **API Tokens** in the admin sidebar. Create tokens with a name, permission level, and optional expiry. Revoke tokens when no longer needed.

**Token permissions**: Stored as a text field (default: `read`).

**Token expiry**: Tokens can be created with an expiry date. Expired tokens are rejected automatically.

### API Endpoints

All endpoints return JSON and require Bearer token authentication.

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/posts` | List published posts |
| GET | `/api/v1/posts/:slug` | Get a single post by slug |
| GET | `/api/v1/pages` | List published pages |
| GET | `/api/v1/pages/:slug` | Get a single page by slug |
| GET | `/api/v1/tags` | List all tags with post counts |
| GET | `/api/v1/tags/:slug` | Get tag details with associated posts |
| GET | `/api/v1/series` | List all series |
| GET | `/api/v1/series/:slug` | Get series details with items |
| GET | `/api/v1/media` | List media files |
| GET | `/api/v1/site` | Get site information |

**Query parameters for list endpoints:**

| Parameter | Description | Default |
|-----------|-------------|---------|
| `page` | Page number | 1 |
| `per_page` | Items per page (capped at `max_page_size`) | `default_page_size` |
| `tag` | Filter posts by tag slug (posts endpoint only) | None |

### API Response Format

**List endpoints** return a data array with pagination metadata:

```json
{
  "data": [ ... ],
  "meta": {
    "total": 42,
    "page": 1,
    "per_page": 20
  }
}
```

**Single item endpoints** return the item directly:

```json
{
  "data": { ... }
}
```

**Errors** return a JSON body with a `message` field and an appropriate HTTP status code (401, 404, etc.).

---

## Webhooks

Webhooks send HTTP POST requests to external URLs when content events occur.

### Setting Up Webhooks

1. Go to **Webhooks** in the admin sidebar
2. Click **Create Webhook**
3. Enter a name, target URL, and optional secret for payload signing
4. Select which events to subscribe to
5. Save

### Webhook Events

| Event | Fires When |
|-------|------------|
| `content.published` | A post or page is created with Published status |
| `content.updated` | A post or page is updated |
| `content.deleted` | A post or page is deleted |
| `media.uploaded` | A file is uploaded to the media library |
| `media.deleted` | A file is deleted from the media library |

### Payload Signing

If you set a secret on the webhook, Pebble signs every payload with HMAC-SHA256 and includes the signature in the `X-Pebble-Signature` header:

```
X-Pebble-Signature: sha256=abc123...
```

Verify the signature on your end by computing `HMAC-SHA256(secret, request_body)` and comparing.

Additional headers included with every delivery:

| Header | Description |
|--------|-------------|
| `Content-Type` | `application/json` |
| `X-Pebble-Event` | The event name (e.g., `content.published`) |
| `X-Pebble-Delivery` | A unique delivery UUID |
| `User-Agent` | `Pebble-CMS-Webhook/1.0` |

### Delivery and Retries

Webhook deliveries happen asynchronously and do not block the admin action that triggered them.

If a delivery fails (non-2xx response), Pebble retries up to 3 times with exponential backoff:
- 1st retry: after 1 second
- 2nd retry: after 4 seconds
- 3rd retry: after 16 seconds

### Delivery Log

View delivery history for each webhook at **Webhooks > Deliveries** in the admin panel. The log shows the event, response status, success/failure, number of attempts, and timestamp.

---

## Content Versioning

### How Versions Work

Every time you update a post or page, Pebble creates a version snapshot of the content before the change is applied. Versions store the Markdown body, title, and metadata at that point in time.

The `version_retention` config setting controls how many versions are kept per content item (default: 50, set to 0 for unlimited).

### Viewing History

In the post or page editor, click **Versions** to see the full version history. Each entry shows the version number, author, and timestamp.

### Comparing Versions

Select two versions to see a side-by-side diff showing added, removed, and modified lines.

### Restoring a Version

Click **Restore** on any version to revert the content to that state. This creates a new version (the current state) before applying the restoration, so no data is lost.

---

## Audit Logging

### What Is Logged

All admin actions are recorded:
- Content creation, updates, and deletion
- User creation and deletion
- Settings changes
- Token creation and revocation
- Webhook management
- Bulk operations
- Login and logout events (if enabled)

Each log entry includes: timestamp, user, action, category, affected entity, and metadata.

### Viewing Audit Logs

Admins can view the audit log at **Audit Log** in the admin sidebar. The dashboard shows a summary of recent activity and a filterable, paginated list of all entries.

### Filtering and Exporting Audit Logs

Filter by user, action type, category, entity type, date range, or search term. Export filtered results as JSON or CSV.

---

## Backup and Restore

### Manual Backups

```bash
pebble backup create                         # Creates backup in ./backups/
pebble backup create -o /mnt/backups         # Custom directory
```

Each backup is a ZIP file named `pebble-backup-{YYYYMMDD_HHMMSS}.zip` containing:
- `pebble.db` -- the complete SQLite database
- `media/` -- all uploaded media files
- `manifest.json` -- metadata (Pebble version, creation time, site title)

### Automatic Backups

Enable automatic backups in `pebble.toml`:

```toml
[backup]
auto_enabled = true
interval_hours = 24         # Backup every 24 hours
retention_count = 7         # Keep the 7 most recent backups
directory = "./backups"     # Where to store backups
```

Automatic backups run in the background during both `pebble serve` and `pebble deploy`.

### Backup Retention

When `retention_count` is set, Pebble automatically deletes the oldest backups beyond the limit after each new backup is created. Only files matching the pattern `pebble-backup-*.zip` in the backup directory are managed.

### Restoring from Backup

```bash
pebble backup restore ./backups/pebble-backup-20250115_120000.zip
```

This restores:
- The database file to the configured `database.path`
- Media files to the configured `media.upload_dir`

**Safety**: The restore operation validates all archive paths to prevent path traversal attacks. Nested paths and `..` sequences are rejected.

> **Tip**: Before restoring, create a backup of the current state so you can revert if needed.

---

## Import and Export

### Importing from WordPress

Export your WordPress site as WXR XML (WordPress admin > Tools > Export > All Content), then:

```bash
pebble import-wp wordpress-export.xml
```

What happens:
- WordPress **posts** become Pebble posts
- WordPress **pages** become Pebble pages
- HTML content is converted to Markdown
- Tags are preserved
- Publish status is mapped (`publish` -> Published, `draft` -> Draft)
- Other post types (attachments, nav_menu_items) are skipped

Use `--overwrite` to replace existing content with matching slugs.

### Importing from Ghost

Export your Ghost site as JSON (Ghost admin > Settings > Labs > Export), then:

```bash
pebble import-ghost ghost-export.json
```

What happens:
- Ghost **posts** become Pebble posts
- Ghost **pages** become Pebble pages
- HTML content is converted to Markdown; Mobiledoc content is extracted first
- Tags are mapped from Ghost's posts_tags junction
- Publish status is mapped (`published` -> Published, `scheduled` -> Scheduled, others -> Draft)

Use `--overwrite` to replace existing content with matching slugs.

### Importing from Pebble Export

```bash
pebble import ./export-directory
pebble import ./export-directory --overwrite
```

Imports Markdown files with YAML frontmatter from a Pebble export directory.

### Exporting Content

```bash
# Pebble native format (YAML frontmatter)
pebble export -o ./export

# Hugo-compatible (TOML frontmatter, Hugo directory structure)
pebble export --format hugo -o ./hugo-site

# Zola-compatible (TOML frontmatter with taxonomies, Zola directory structure)
pebble export --format zola -o ./zola-site

# Include everything
pebble export --format hugo --include-drafts --include-media -o ./full-export
```

---

## Static Site Generation

### Building a Static Site

```bash
pebble build --output ./public --base-url https://example.com
```

Generates a complete, self-contained static site with all pages, feeds, and media.

### Output Structure

```
public/
  index.html                    Homepage
  feed.xml                      RSS 2.0 feed
  feed.json                     JSON Feed
  sitemap.xml                   Sitemap with image entries
  posts/
    index.html                  Posts listing (page 1)
    page/2/index.html           Posts listing (page 2, etc.)
    my-post/index.html          Individual posts
  my-page/index.html            Individual pages
  tags/
    index.html                  All tags listing
    my-tag/index.html           Posts filtered by tag
    my-tag/feed.xml             Tag-specific RSS feed
  search/
    index.html                  Client-side search page
    index.json                  Search index (JSON)
  media/                        All uploaded media files
```

### Deployment Options

The generated directory works with any static hosting:

- **GitHub Pages**: Push to a `gh-pages` branch or use GitHub Actions
- **Netlify / Vercel / Cloudflare Pages**: Point to the build output directory
- **AWS S3 + CloudFront**: Upload to an S3 bucket with static hosting enabled
- **Any web server**: Copy the files to your nginx, Apache, or Caddy document root

---

## Multi-Site Registry

### Registry Overview

The registry lets you manage multiple Pebble sites from a single installation. Sites are stored under `~/.pebble/registry/` with independent databases, configurations, and ports.

### Creating Registry Sites

```bash
pebble registry init blog --title "Personal Blog"
pebble registry init docs --title "Documentation"
```

Each site gets its own directory under `~/.pebble/registry/{name}/` with a full Pebble setup.

### Starting and Stopping Sites

```bash
# Start in development mode (with admin panel)
pebble registry serve blog
pebble registry serve blog -p 3005          # Specific port

# Start in production mode (admin disabled)
pebble registry deploy blog
pebble registry deploy blog -p 8080

# Check status
pebble registry status blog
pebble registry list                         # All sites with status and ports

# Stop
pebble registry stop blog
pebble registry stop-all
```

Ports are auto-assigned from the range 3001-3100 (configurable in global settings) if not specified.

### Registry Site Configuration

```bash
# View all config
pebble registry config mysite

# Get a value (dot notation)
pebble registry config mysite get theme.name

# Set a value
pebble registry config mysite set theme.name minimal
pebble registry config mysite set content.posts_per_page 20
pebble registry config mysite set theme.custom.primary_color "#e63946"

# Open in your $EDITOR
pebble registry config mysite edit
```

If the site is running when you change its configuration, it automatically restarts to apply the changes.

### Site Logs

Registry sites write logs to `~/.pebble/registry/{name}/logs/{name}.log`:

```bash
# View recent logs
tail ~/.pebble/registry/blog/logs/blog.log

# Follow logs in real-time
tail -f ~/.pebble/registry/blog/logs/blog.log
```

---

## Feeds and Discovery

### RSS Feed

Available at `/feed.xml`. Includes all published posts in RSS 2.0 format.

### JSON Feed

Available at `/feed.json`. Includes all published posts in JSON Feed format.

### Tag RSS Feeds

Each tag has its own RSS feed at `/tags/{slug}/feed.xml`. Includes the 20 most recent posts with that tag.

### Sitemap

Available at `/sitemap.xml`. Includes all published posts and pages with `<lastmod>` dates. Posts with featured images include `<image:image>` entries for search engine image discovery.

### Health Check

```
GET /health
```

Returns HTTP 200 with `{"status": "healthy", "version": "0.9.5"}` when the database is accessible, or HTTP 503 with `{"status": "unhealthy", ...}` otherwise. Use this with reverse proxies, load balancers, or uptime monitors.

### Draft Previews

Share unpublished content with reviewers using time-limited preview URLs.

1. Open a draft post or page in the admin editor
2. Click the **Preview** button
3. A unique preview URL is generated, valid for 1 hour
4. Share the URL -- no login required to view

Preview URLs follow the format: `https://yoursite.com/preview/{token}`

---

## Security

### Rate Limiting

| Scope | Limit | Lockout Duration |
|-------|-------|-----------------|
| Login attempts | 5 per 5 minutes | 15 minutes |
| File uploads | 20 per 60 seconds | 5 minutes |
| Admin write operations | 30 per 60 seconds | 5 minutes |

Exceeding a rate limit returns `429 Too Many Requests`.

### Session Security

- **HttpOnly cookies**: Session tokens are not accessible to JavaScript
- **Secure flag**: Enforced in production (HTTPS only)
- **SameSite=Strict**: Prevents cross-origin request forgery
- **Session rotation**: Old sessions are invalidated on login
- **Configurable lifetime**: Default 7 days, configurable via `auth.session_lifetime`

### Content Security Policy

Pebble sets security headers on all responses:
- `Content-Security-Policy` restricting script, style, image, and frame sources
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `X-XSS-Protection: 1; mode=block`
- `Referrer-Policy: strict-origin-when-cross-origin`

### CSRF Protection

All admin forms include CSRF tokens. Requests without a valid token are rejected with a 403 response.

### SVG Sanitization

Uploaded SVG files are scanned for malicious content. Files containing `<script>`, `javascript:` URLs, event handlers (`onload`, `onerror`, etc.), `eval()`, or other dangerous patterns are rejected.

---

## Performance

### SQLite Tuning

Pebble applies production-safe SQLite settings automatically:

| Setting | Value | Purpose |
|---------|-------|---------|
| `journal_mode` | WAL | Concurrent reads during writes |
| `busy_timeout` | 5000 ms | Wait on lock contention |
| `journal_size_limit` | 64 MB | Cap WAL file growth |
| `synchronous` | NORMAL | Safe with WAL, faster than FULL |
| `mmap_size` | 128 MB | Memory-mapped I/O for reads |
| `cache_size` | ~64 MB | In-memory page cache |
| `foreign_keys` | ON | Enforce referential integrity |

### Connection Pooling

Pebble uses r2d2 connection pooling with a default pool size of 10 connections, configurable via `database.pool_size`.

### Graceful Shutdown

On `SIGTERM` or `Ctrl+C`, Pebble drains all in-flight requests before exiting. No interrupted responses during deployments or restarts.

---

## Global Configuration

### Config File Location

Global settings are stored in `~/.pebble/config.toml` and managed through `pebble config`.

### Available Global Settings

| Key | Description | Default |
|-----|-------------|---------|
| `defaults.author` | Default author name | System username |
| `defaults.language` | Default language code | `en` |
| `defaults.theme` | Default theme for new sites | `default` |
| `defaults.posts_per_page` | Default posts per page | `10` |
| `defaults.excerpt_length` | Default excerpt character limit | `200` |
| `defaults.dev_port` | Default development server port | `3000` |
| `defaults.prod_port` | Default production server port | `8080` |
| `registry.auto_port_range_start` | Auto-assign ports from | `3001` |
| `registry.auto_port_range_end` | Auto-assign ports to | `3100` |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Logging verbosity (e.g., `info`, `debug`, `pebble=debug,tower_http=debug`) |

---

## Troubleshooting

### "Could not read config file"

You're not in a directory containing `pebble.toml`. Either `cd` to your site directory or specify the path:

```bash
pebble -c /path/to/pebble.toml serve
```

### Database locked

Only one Pebble process should access a database at a time. Check for running processes:

```bash
pebble registry list
pebble registry stop-all
```

Or check for other processes using the database file.

### Port already in use

Specify a different port:

```bash
pebble serve --port 3001
```

Or let the registry auto-assign an available port:

```bash
pebble registry serve mysite
```

### Forgotten admin password

Reset a user's password from the command line:

```bash
pebble user passwd admin-username
```

This prompts for a new password interactively.

### Media uploads failing

Check that:
1. The `media.upload_dir` directory exists and is writable
2. The file is under the 50 MB size limit
3. The file type is in the supported list (JPEG, PNG, GIF, WebP, SVG, MP4, WebM, MP3, OGG, PDF)
4. You haven't exceeded the upload rate limit (20 uploads per 60 seconds)

### After upgrading Pebble

Run migrations to apply any schema changes:

```bash
pebble migrate
```

Then re-render content to pick up any Markdown renderer improvements:

```bash
pebble rerender
```

### Static build missing content

The `pebble build` command only includes published content. Make sure your posts and pages have the Published status before building.

### Webhooks not firing

1. Verify the webhook is set to **Active** in the admin panel
2. Verify the webhook is subscribed to the correct events
3. Check the **Delivery Log** for the webhook to see error details
4. Ensure the `webhooks` feature is enabled (it's a default feature)

### API returning 401

1. Verify that `api.enabled = true` in `pebble.toml`
2. Check that the token hasn't expired
3. Verify the `Authorization: Bearer pb_...` header format is correct
4. Check if the token has been revoked in the admin panel
