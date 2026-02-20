# Pebble CMS Usage Guide

Pebble is a lightweight personal CMS that can run as a dynamic server or generate static sites.

## Table of Contents

- [Quick Start](#quick-start)
- [CLI Commands](#cli-commands)
- [Configuration](#configuration)
- [Content Management](#content-management)
- [Content Series](#content-series)
- [Snippets](#snippets)
- [Media Management](#media-management)
- [Markdown Editor](#markdown-editor)
- [Shortcodes](#shortcodes)
- [Bulk Operations](#bulk-operations)
- [Themes](#themes)
- [RSS Feeds](#rss-feeds)
- [Health Check](#health-check)
- [Draft Preview](#draft-preview)
- [User Management](#user-management)
- [Backup and Restore](#backup-and-restore)
- [Static Site Generation](#static-site-generation)
- [Global Configuration](#global-configuration)
- [Site Registry](#site-registry)
- [Security](#security)
- [Performance](#performance)

## Quick Start

```bash
# Create a new site
pebble init mysite
cd mysite

# Start the development server
pebble serve

# Open http://127.0.0.1:3000/admin to set up your first user
```

## CLI Commands

### `pebble init [PATH]`

Initialize a new Pebble site.

```bash
pebble init                    # Initialize in current directory
pebble init mysite             # Initialize in ./mysite
pebble init --name "My Blog"   # Set the site name
```

Creates `pebble.toml` configuration and initializes the database.

### `pebble serve`

Start the development server.

```bash
pebble serve                         # Default: 127.0.0.1:3000
pebble serve --host 0.0.0.0          # Listen on all interfaces
pebble serve --port 8080             # Use port 8080
pebble serve -H 0.0.0.0 -p 8080      # Combined
```

The development server binds to `127.0.0.1` by default (localhost only).

### `pebble deploy`

Start the production server.

```bash
pebble deploy                        # Default: 0.0.0.0:8080
pebble deploy --host 127.0.0.1       # Localhost only
pebble deploy --port 3000            # Use port 3000
```

The production server binds to `0.0.0.0` by default (all interfaces). Supports graceful shutdown via `SIGTERM` or `Ctrl+C` — in-flight requests are drained before the process exits.

### `pebble build`

Generate a static site.

```bash
pebble build                              # Output to ./dist
pebble build --output ./public            # Custom output directory
pebble build --base-url https://example.com  # Set base URL for links
```

Generates:
- HTML pages for all posts and pages
- Tag archive pages
- Search page with JSON index
- RSS feed (`feed.xml`)
- JSON Feed (`feed.json`)
- Tag-scoped RSS feeds (`tags/{slug}/feed.xml`)
- Sitemap with image entries (`sitemap.xml`)
- Copied media files

### `pebble export`

Export content to JSON files.

```bash
pebble export                    # Export to ./export
pebble export --output ./backup  # Custom directory
pebble export --include-drafts   # Include draft content
pebble export --include-media    # Include media files
```

### `pebble import [PATH]`

Import content from JSON files.

```bash
pebble import                    # Import from ./export
pebble import ./backup           # Import from specific directory
pebble import --overwrite        # Overwrite existing content with same slugs
```

### `pebble migrate`

Run database migrations to update the schema.

```bash
pebble migrate
```

Run this after upgrading Pebble to apply any schema changes.

## Configuration

Pebble is configured via `pebble.toml` in the site root.

### Complete Configuration Reference

```toml
[site]
title = "My Site"
description = "A personal blog"
url = "http://localhost:3000"
language = "en"

[server]
host = "127.0.0.1"
port = 3000

[database]
path = "./data/pebble.db"

[content]
posts_per_page = 10          # 1-100
excerpt_length = 200         # 1-10000
auto_excerpt = true          # Auto-generate excerpts from content

[media]
upload_dir = "./data/media"
max_upload_size = "10MB"

[theme]
name = "default"             # See Themes section for all 15 options

[theme.custom]
# Override theme colors (optional)
primary_color = "#007acc"
primary_color_hover = "#005a9e"
accent_color = "#ff6b6b"
background_color = "#ffffff"
background_secondary = "#f5f5f5"
text_color = "#333333"
text_muted = "#666666"
border_color = "#e0e0e0"
link_color = "#007acc"
font_family = "system-ui, -apple-system, sans-serif"
heading_font_family = "Georgia, serif"
font_size = "16px"
line_height = 1.6
border_radius = "4px"

[auth]
session_lifetime = "7d"      # Session duration (e.g., "7d", "24h")

[homepage]
show_hero = true
hero_layout = "centered"     # centered, split, minimal
hero_height = "medium"       # small, medium, large, full
hero_text_align = "center"   # left, center, right
hero_image = "/media/hero.jpg"  # Optional hero background
show_pages = true
show_posts = true
posts_layout = "grid"        # grid, list
posts_columns = 2            # 1-4, for grid layout
pages_layout = "grid"        # grid, list
sections_order = ["hero", "pages", "posts"]  # Custom section order
```

## Content Management

### Content Types

Pebble supports three content types:

- **Posts**: Blog posts, displayed in chronological order with pagination
- **Pages**: Static pages, accessible at `/{slug}`
- **Snippets**: Reusable content blocks embeddable via shortcode (see [Snippets](#snippets))

### Content Status

- **Draft**: Not visible on the public site
- **Scheduled**: Will be published at `scheduled_at` time
- **Published**: Visible on the public site
- **Archived**: Hidden from listings but accessible via direct URL

### Creating Content

Access the admin interface at `/admin` to create and edit content.

Posts and pages support:
- Markdown with syntax highlighting
- Featured images
- Custom excerpts (or auto-generated)
- Tags (posts only)
- Scheduled publishing
- SEO metadata
- Content versioning (automatic version snapshots on update)
- Audit logging (all changes tracked)

### Markdown Features

Pebble's Markdown renderer supports:
- Tables
- Footnotes
- Strikethrough
- Task lists
- Syntax-highlighted code blocks

Code blocks support many languages including: rust, python, javascript, typescript, go, c, cpp, java, html, css, json, yaml, toml, sql, bash, shell, and markdown.

````markdown
```rust
fn main() {
    println!("Hello, Pebble!");
}
```
````

## Content Series

Series let you group posts into ordered sequences (e.g., "Building X in Rust, Part 1-5") with automatic prev/next navigation.

### Managing Series

1. Navigate to `/admin/series` in the admin panel
2. Click **New Series**
3. Fill in the title, slug (auto-generated if left blank), and description
4. Set the status to **Draft** or **Published**
5. Add posts using the "Add Post" dropdown — posts can be from any status
6. Drag and drop to reorder posts within the series
7. Click **Create Series** to save

### Public Series Page

Published series are accessible at `/series/{slug}`. The series page displays:
- Series title and description
- An ordered, numbered list of all posts in the series
- Published posts link to their full content; unpublished posts are shown greyed out

### Series Navigation on Posts

When a post belongs to a published series, a navigation bar automatically appears below the post content showing:
- Which part of the series this post is (e.g., "Part 2 of 5")
- A link back to the series overview page
- Previous and next post links within the series

## Snippets

Snippets are reusable content blocks that you manage in the admin and embed in posts or pages via shortcode.

### Managing Snippets

1. Navigate to `/admin/snippets` in the admin panel
2. Click **New Snippet**
3. Enter a title and slug (the slug is used in the embed shortcode)
4. Write the snippet content in the Markdown editor — live preview is available
5. Click **Create Snippet**

When editing an existing snippet, the form shows the embed shortcode you can copy:

```
[snippet slug="your-slug"]
```

### Embedding Snippets

Place the shortcode anywhere in your post or page markdown:

```markdown
Check out our standard disclaimer:

[snippet slug="disclaimer"]

And here's the rest of my post...
```

## Media Management

### Supported File Types

- **Images**: JPEG, PNG, GIF, WebP, SVG
- **Video**: MP4, WebM
- **Audio**: MP3, OGG
- **Documents**: PDF

Maximum upload size: 10MB per file.

### Image Optimization

Uploaded images are automatically:
- Optimized for web
- Converted to WebP format (with original preserved)
- Responsive variants generated at 400w, 800w, 1200w, and 1600w breakpoints
- Thumbnails generated for galleries

### Responsive Images

When images are embedded via shortcodes, Pebble automatically serves responsive `<picture>` elements with `srcset` and WebP sources. The browser selects the best size for the viewer's screen:

```html
<picture>
  <source srcset="/media/photo-400w.webp 400w,
                  /media/photo-800w.webp 800w,
                  /media/photo-1200w.webp 1200w,
                  /media/photo.webp 1600w"
          sizes="(max-width: 400px) 400px,
                 (max-width: 800px) 800px,
                 (max-width: 1200px) 1200px,
                 1600px"
          type="image/webp">
  <img src="/media/photo.jpg" alt="description" loading="lazy">
</picture>
```

### Uploading Media

Upload media through the admin interface at `/admin/media`. Files are stored in the configured `upload_dir`.

You can also upload images directly from the editor:
- **Drag-and-drop**: Drag image files onto the editor textarea
- **Paste from clipboard**: Copy an image (e.g., a screenshot) and paste with Ctrl/Cmd+V

Both methods automatically upload to the media library and insert the Markdown image syntax at the cursor position.

## Markdown Editor

The admin editor includes a rich set of productivity features.

### Toolbar

A formatting toolbar appears above the editor with buttons for:

| Button | Action | Inserts |
|--------|--------|---------|
| **B** | Bold | `**text**` |
| *I* | Italic | `*text*` |
| Link | Link | `[text](url)` |
| H | Heading | `## ` |
| `</>` | Inline code | `` `text` `` |
| Quote | Blockquote | `> ` |
| Bullet | Bullet list | `- ` |
| 1. | Numbered list | `1. ` |
| Image | Image | `![alt text](image-url)` |

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Cmd/Ctrl + S | Save the current form |
| Cmd/Ctrl + Shift + P | Set status to Published and save |
| Cmd/Ctrl + B | Bold selection |
| Cmd/Ctrl + I | Italic selection |
| Cmd/Ctrl + K | Wrap selection as link |

### Auto-Save Drafts

The editor automatically saves your work to the browser's local storage every 2 seconds while you type. If you navigate away or close the browser accidentally, the next time you open the editor it will offer to restore the unsaved draft.

- Drafts are saved per-page (based on the URL path)
- Includes both the title and body fields
- Shows a "Draft saved" indicator when saving occurs
- Automatically cleared on successful form submission

### Image Upload in Editor

**Drag and drop**: Drag any image file from your computer onto the editor textarea. The image uploads to your media library and a Markdown image tag is inserted automatically.

**Paste to upload**: Copy an image to your clipboard (screenshot, image from a web page, etc.) and paste into the editor with Cmd/Ctrl+V. The image is uploaded and inserted just like drag and drop.

During upload, a `![Uploading...]()` placeholder appears. Once complete, it's replaced with the actual image path.

## Shortcodes

Embed media in your content using shortcodes.

### `[image]` - Embed an Image

```markdown
[image src="photo.jpg"]
[image src="photo.jpg" alt="A beautiful sunset"]
[image src="photo.jpg" alt="Sunset" title="Summer 2024" width="800"]
```

Attributes:
- `src` (required): Filename of uploaded media
- `alt`: Alternative text for accessibility
- `title`: Title tooltip
- `width`, `height`: Dimensions
- `class`: CSS class (default: `media-image`)

Images automatically render with responsive `<picture>` elements and WebP variants when available.

### `[video]` - Embed a Video

```markdown
[video src="demo.mp4"]
[video src="demo.mp4" controls]
[video src="demo.mp4" autoplay muted loop]
[video src="demo.mp4" poster="thumbnail.jpg" width="1280" height="720"]
```

Attributes:
- `src` (required): Filename of uploaded video
- `controls`: Show player controls (default: on)
- `nocontrols`: Hide player controls
- `autoplay`: Auto-play video (requires `muted`)
- `muted`: Mute audio
- `loop`: Loop playback
- `poster`: Thumbnail image filename
- `width`, `height`: Dimensions
- `class`: CSS class (default: `media-video`)

### `[audio]` - Embed Audio

```markdown
[audio src="podcast.mp3"]
[audio src="music.mp3" controls loop]
```

Attributes:
- `src` (required): Filename of uploaded audio
- `controls`: Show player controls (default: on)
- `nocontrols`: Hide player controls
- `autoplay`: Auto-play audio
- `loop`: Loop playback
- `class`: CSS class (default: `media-audio`)

### `[media]` - Auto-Detect Type

```markdown
[media src="file.jpg"]    # Renders as image
[media src="video.mp4"]   # Renders as video
[media src="audio.mp3"]   # Renders as audio
[media src="doc.pdf"]     # Renders as embedded PDF
```

Automatically detects the media type based on file extension and renders appropriately.

### `[gallery]` - Image Gallery

```markdown
[gallery src="img1.jpg,img2.jpg,img3.jpg"]
[gallery src="img1.jpg,img2.jpg,img3.jpg,img4.jpg" columns="4"]
```

Attributes:
- `src` (required): Comma-separated list of image filenames
- `columns`: Number of columns (default: 3)
- `class`: CSS class (default: `media-gallery`)

## Bulk Operations

The posts list at `/admin/posts` supports bulk actions for managing multiple posts at once.

### How to Use

1. Navigate to **Posts** in the admin sidebar
2. Use the checkboxes on the left to select individual posts, or click the header checkbox to select all
3. A bulk action bar appears showing how many posts are selected
4. Click one of the action buttons:

| Action | Effect |
|--------|--------|
| **Publish** | Sets selected posts to Published status |
| **Unpublish** | Sets selected posts back to Draft status |
| **Archive** | Sets selected posts to Archived status |
| **Delete** | Permanently deletes selected posts (with confirmation prompt) |

All bulk operations are recorded in the audit log.

## Themes

### Available Themes

Pebble ships with 15 built-in themes:

| Theme | Description |
|-------|-------------|
| **default** | Clean, modern design |
| **minimal** | Stripped-down, content-focused |
| **magazine** | Multi-column, editorial style |
| **brutalist** | Bold, unconventional aesthetics |
| **neon** | Dark theme with vibrant colors |
| **serif** | Classic, serif-focused typography |
| **ocean** | Cool, blue-toned palette |
| **midnight** | Dark theme with deep tones |
| **botanical** | Nature-inspired, earthy colors |
| **monochrome** | Black and white, no frills |
| **coral** | Warm, coral-accented palette |
| **terminal** | Dark theme with monospace, green-on-black aesthetic |
| **nordic** | Clean, Scandinavian-inspired design |
| **sunset** | Warm gradient tones |
| **typewriter** | Classic typewriter-inspired monospace look |

Set the theme in `pebble.toml`:

```toml
[theme]
name = "minimal"
```

All themes support both light and dark mode. Dark-first themes (neon, midnight, terminal) use dark mode by default and override styles for light mode.

### Custom Theme Colors

Override any theme's colors in `pebble.toml`:

```toml
[theme]
name = "default"

[theme.custom]
primary_color = "#e63946"
background_color = "#f1faee"
text_color = "#1d3557"
```

### Themed Error Pages

404 and 500 error pages automatically use the active theme's styling, so errors feel consistent with the rest of your site.

## RSS Feeds

### Site-Wide Feed

The main RSS feed is available at `/feed.xml` and the JSON Feed at `/feed.json`. Both include all published posts.

### Tag-Scoped RSS Feeds

Readers can subscribe to specific topics via per-tag RSS feeds:

```
https://yoursite.com/tags/rust/feed.xml
https://yoursite.com/tags/tutorials/feed.xml
```

Each tag feed includes the 20 most recent posts with that tag, formatted as RSS 2.0. The feed title is formatted as "{Tag Name} - {Site Title}".

## Health Check

Pebble exposes a health check endpoint for use with reverse proxies, load balancers, and uptime monitors.

**Endpoint**: `GET /health`

**Response when healthy** (HTTP 200):
```json
{
  "status": "healthy",
  "version": "0.9.0"
}
```

**Response when unhealthy** (HTTP 503):
```json
{
  "status": "unhealthy",
  "version": "0.9.0"
}
```

The health check verifies database connectivity by executing a test query.

## Draft Preview

Share unpublished content with reviewers using signed, time-limited preview URLs.

### Generating a Preview Link

1. Open a draft or scheduled post in the admin editor
2. Click the **Preview** button (or send a POST request to `/admin/preview/{id}`)
3. A unique preview URL is generated, valid for **1 hour**

### Preview URL Format

```
https://yoursite.com/preview/{token}
```

- Tokens are cryptographically random, URL-safe strings
- Each token is single-use per generation (regenerate for a fresh link)
- Expired tokens return a 404 error
- The preview page renders the content exactly as it would appear when published

## User Management

### Roles

- **Admin**: Full access to all features
- **Author**: Can create and edit own content
- **Viewer**: Read-only access to admin

### CLI User Commands

```bash
# Add a user
pebble user add --username alice --email alice@example.com --role admin
pebble user add --username bob --email bob@example.com --role author --password secret123

# List users
pebble user list

# Change password (prompts for new password)
pebble user passwd alice

# Remove a user
pebble user remove alice
```

### First-Time Setup

When no users exist, visiting `/admin` shows a setup page to create the first admin account.

## Backup and Restore

### Create Backup

```bash
pebble backup create                     # Save to ./backups
pebble backup create --output ./archive  # Custom directory
```

Creates a timestamped backup file containing the database and media files.

### List Backups

```bash
pebble backup list                    # List from ./backups
pebble backup list --dir ./archive    # List from custom directory
```

### Restore Backup

```bash
pebble backup restore ./backups/pebble-backup-2024-01-15-120000.tar.gz
```

Restores the database and media files from a backup archive.

## Static Site Generation

Generate a fully static version of your site:

```bash
pebble build --output ./public --base-url https://example.com
```

### Generated Files

```
public/
├── index.html              # Homepage
├── feed.xml                # RSS feed
├── feed.json               # JSON Feed
├── sitemap.xml             # Sitemap (with image entries)
├── posts/
│   ├── index.html          # Posts listing
│   ├── page/2/index.html   # Pagination
│   └── my-post/index.html  # Individual posts
├── my-page/index.html      # Individual pages
├── series/
│   └── my-series/index.html  # Series overview pages
├── tags/
│   ├── index.html          # All tags
│   ├── my-tag/index.html   # Posts by tag
│   └── my-tag/feed.xml     # Tag RSS feed
├── search/
│   ├── index.html          # Search page
│   └── index.json          # Search index
└── media/                  # Uploaded media files
```

### Sitemap

The generated `sitemap.xml` includes `<image:image>` entries for posts with featured images, improving image discoverability by search engines.

### Deployment

The generated `./public` directory can be deployed to any static hosting:
- GitHub Pages
- Netlify
- Vercel
- Cloudflare Pages
- AWS S3 + CloudFront
- Any web server (nginx, Apache)

## Global Configuration

Pebble stores global settings in `~/.pebble/config.toml`.

### Config Commands

```bash
# List all settings
pebble config list

# Get a specific setting
pebble config get defaults.author

# Set a value
pebble config set defaults.theme minimal
pebble config set defaults.posts_per_page 15

# Remove a custom value (resets to default)
pebble config remove custom.my_key

# Show config file path
pebble config path
```

### Available Settings

| Key | Description | Default |
|-----|-------------|---------|
| `defaults.author` | Default author name | System username |
| `defaults.language` | Default language code | `en` |
| `defaults.theme` | Default theme for new sites | `default` |
| `defaults.posts_per_page` | Posts per page | `10` |
| `defaults.excerpt_length` | Excerpt character limit | `200` |
| `defaults.dev_port` | Development server port | `3000` |
| `defaults.prod_port` | Production server port | `8080` |
| `registry.auto_port_range_start` | Auto-assign ports from | `3001` |
| `registry.auto_port_range_end` | Auto-assign ports to | `3100` |

## Site Registry

The registry manages multiple Pebble sites from a centralized location (`~/.pebble/registry/`).

### Registry Commands

```bash
# Create a new site
pebble registry init mysite
pebble registry init myblog --title "My Blog"

# List all registered sites
pebble registry list

# Start a site (development mode)
pebble registry serve mysite
pebble registry serve mysite --port 3005

# Start a site (production mode)
pebble registry deploy mysite
pebble registry deploy mysite --port 8080

# Check site status
pebble registry status mysite

# Stop a running site
pebble registry stop mysite

# Stop all running sites
pebble registry stop-all

# View/edit site configuration
pebble registry config mysite              # View all config
pebble registry config mysite get <key>    # Get a value
pebble registry config mysite set <key> <value>  # Set a value
pebble registry config mysite edit         # Open in editor

# Remove a site from registry
pebble registry remove mysite

# Show registry directory path
pebble registry path
```

### How It Works

1. `registry init` creates a site in `~/.pebble/registry/{name}/` with its own database and configuration
2. `registry serve/deploy` spawns a background Pebble process and tracks its PID
3. Ports are automatically assigned from the configured range if not specified
4. `registry list` shows all sites with their current status and ports
5. Commands can be run from any directory - Pebble routes to the correct site

### Example Workflow

```bash
# Create two sites
pebble registry init blog --title "Personal Blog"
pebble registry init docs --title "Documentation"

# Start both
pebble registry serve blog
pebble registry serve docs

# Check status
pebble registry list
# NAME       STATUS    PORT   TITLE
# blog       running   3001   Personal Blog
# docs       running   3002   Documentation

# Stop all when done
pebble registry stop-all
```

### Site Configuration

View and edit configuration for registry sites using `pebble registry config`.

#### View Full Configuration

```bash
pebble registry config mysite
```

Displays all configuration values grouped by section:

```
# Site
site.title                      My Site
site.description                A Pebble site
site.url                        http://localhost:3000
site.language                   en

# Server
server.host                     127.0.0.1
server.port                     3000

# Content
content.posts_per_page          10
content.excerpt_length          200
content.auto_excerpt            true

# Theme
theme.name                      default

# Homepage
homepage.show_hero              true
homepage.hero_layout            centered
homepage.show_posts             true
homepage.posts_layout           grid
homepage.show_pages             true
```

#### Get a Specific Value

```bash
pebble registry config mysite get <key>
```

Examples:

```bash
pebble registry config mysite get theme.name
# default

pebble registry config mysite get site.title
# My Site

pebble registry config mysite get content.posts_per_page
# 10
```

#### Set a Value

```bash
pebble registry config mysite set <key> <value>
```

Examples:

```bash
# Change theme
pebble registry config mysite set theme.name minimal

# Update site title
pebble registry config mysite set site.title "My Personal Blog"

# Change posts per page
pebble registry config mysite set content.posts_per_page 20

# Disable hero section
pebble registry config mysite set homepage.show_hero false

# Change server port
pebble registry config mysite set server.port 3005

# Set custom theme color
pebble registry config mysite set theme.custom.primary_color "#e63946"
```

If the site is running, it automatically restarts to apply the changes.

#### Open in Editor

```bash
pebble registry config mysite edit
```

Opens the site's `pebble.toml` in your default editor (`$EDITOR`, or `vi` on Unix / `notepad` on Windows). The configuration is validated after saving, and the site restarts if it was running.

#### Available Configuration Keys

| Key | Type | Description |
|-----|------|-------------|
| `site.title` | string | Site title |
| `site.description` | string | Site description |
| `site.url` | string | Site URL |
| `site.language` | string | Language code (e.g., `en`) |
| `server.host` | string | Server bind address |
| `server.port` | number | Server port |
| `content.posts_per_page` | number | Posts per page (1-100) |
| `content.excerpt_length` | number | Excerpt length (1-10000) |
| `content.auto_excerpt` | boolean | Auto-generate excerpts |
| `theme.name` | string | Theme name (see [Themes](#themes) for all 15 options) |
| `theme.custom.primary_color` | string | Primary color (hex) |
| `theme.custom.accent_color` | string | Accent color (hex) |
| `theme.custom.background_color` | string | Background color (hex) |
| `theme.custom.text_color` | string | Text color (hex) |
| `theme.custom.font_family` | string | Font family |
| `homepage.show_hero` | boolean | Show hero section |
| `homepage.hero_layout` | string | Hero layout (`centered`, `split`, `minimal`) |
| `homepage.hero_height` | string | Hero height (`small`, `medium`, `large`, `full`) |
| `homepage.show_posts` | boolean | Show posts section |
| `homepage.posts_layout` | string | Posts layout (`grid`, `list`) |
| `homepage.posts_columns` | number | Grid columns (1-4) |
| `homepage.show_pages` | boolean | Show pages section |
| `homepage.pages_layout` | string | Pages layout (`grid`, `list`) |
| `auth.session_lifetime` | string | Session duration (e.g., `7d`, `24h`) |

### Site Logs

Registry sites write logs to `~/.pebble/registry/{name}/logs/{name}.log`. View logs with:

```bash
# View recent logs
tail ~/.pebble/registry/mysite/logs/mysite.log

# Follow logs in real-time
tail -f ~/.pebble/registry/mysite/logs/mysite.log
```

## Security

Pebble includes several security hardening features out of the box.

### Rate Limiting

All write endpoints are rate-limited to prevent abuse:

| Scope | Limit | Lockout |
|-------|-------|---------|
| Login attempts | 5 per 5 minutes | 15 minutes |
| File uploads | 20 per 60 seconds | 5 minutes |
| Admin write operations | 30 per 60 seconds | 5 minutes |

Write rate limiting covers all POST/DELETE operations under `/admin/*` including content creation, media uploads, settings changes, and user management. Exceeding the limit returns a `429 Too Many Requests` response.

### Session Security

- **HttpOnly cookies** prevent JavaScript access to session tokens
- **Secure flag** enforced in production (HTTPS only)
- **SameSite=Strict** attribute prevents CSRF via cross-origin requests
- **Session rotation** on login (old sessions are invalidated)
- **7-day max age** with automatic expiration

### Content Security Policy

Pebble sets the following security headers on all responses:

- `Content-Security-Policy` restricting script, style, image, and frame sources
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `X-XSS-Protection: 1; mode=block`
- `Referrer-Policy: strict-origin-when-cross-origin`

### CSRF Protection

All admin forms include CSRF tokens. Requests without a valid token are rejected.

### Audit Logging

All admin actions (content creation, updates, deletions, settings changes, bulk operations) are recorded in the audit log with timestamps and user information.

## Performance

### SQLite Tuning

Pebble applies production-safe SQLite defaults automatically:

| Setting | Value | Purpose |
|---------|-------|---------|
| `journal_mode` | WAL | Concurrent reads during writes |
| `busy_timeout` | 5000ms | Wait on lock contention instead of failing |
| `journal_size_limit` | 64 MB | Cap WAL file growth |
| `synchronous` | NORMAL | Safe with WAL, faster than FULL |
| `mmap_size` | 128 MB | Memory-mapped I/O for faster reads |
| `cache_size` | ~64 MB | In-memory page cache |
| `foreign_keys` | ON | Enforce referential integrity |

### Graceful Shutdown

When receiving `SIGTERM` or `Ctrl+C`, Pebble drains all in-flight requests before exiting. This prevents interrupted responses during deployments or restarts.

## Environment Variables

Pebble respects these environment variables:

| Variable | Description |
|----------|-------------|
| `PEBBLE_CONFIG` | Path to config file (alternative to `-c` flag) |
| `RUST_LOG` | Logging level (e.g., `info`, `debug`, `pebble=debug`) |

## Troubleshooting

### "Could not read config file"

Make sure you're in a Pebble site directory containing `pebble.toml`, or specify the path:

```bash
pebble -c /path/to/pebble.toml serve
```

### Database locked

Only one Pebble process can access the database at a time. Check for running processes:

```bash
pebble registry list
pebble registry stop-all
```

### Port already in use

Specify a different port:

```bash
pebble serve --port 3001
```

Or let the registry auto-assign:

```bash
pebble registry serve mysite
```
