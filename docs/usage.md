# Pebble CMS Usage Guide

Pebble is a lightweight personal CMS that can run as a dynamic server or generate static sites.

## Table of Contents

- [Quick Start](#quick-start)
- [CLI Commands](#cli-commands)
- [Configuration](#configuration)
- [Content Management](#content-management)
- [Media Management](#media-management)
- [Shortcodes](#shortcodes)
- [Themes](#themes)
- [User Management](#user-management)
- [Backup and Restore](#backup-and-restore)
- [Static Site Generation](#static-site-generation)
- [Global Configuration](#global-configuration)
- [Site Registry](#site-registry)

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

The production server binds to `0.0.0.0` by default (all interfaces).

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
- Sitemap (`sitemap.xml`)
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
name = "default"             # default, minimal, magazine, brutalist, neon

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
- **Snippets**: Reusable content blocks (internal use)

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
- Thumbnails generated for galleries

### Uploading Media

Upload media through the admin interface at `/admin/media`. Files are stored in the configured `upload_dir`.

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

## Themes

### Available Themes

- **default**: Clean, modern design
- **minimal**: Stripped-down, content-focused
- **magazine**: Multi-column, editorial style
- **brutalist**: Bold, unconventional aesthetics
- **neon**: Dark theme with vibrant colors

Set the theme in `pebble.toml`:

```toml
[theme]
name = "minimal"
```

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
├── sitemap.xml             # Sitemap
├── posts/
│   ├── index.html          # Posts listing
│   ├── page/2/index.html   # Pagination
│   └── my-post/index.html  # Individual posts
├── my-page/index.html      # Individual pages
├── tags/
│   ├── index.html          # All tags
│   └── my-tag/index.html   # Posts by tag
├── search/
│   ├── index.html          # Search page
│   └── index.json          # Search index
└── media/                  # Uploaded media files
```

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
