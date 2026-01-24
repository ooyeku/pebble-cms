# Pebble CMS Guide

A lightweight, single-binary personal CMS built with Rust.

## Quick Start

```bash
# Build Pebble
cargo build --release

# Create a new site
./target/release/pebble init mysite --name "My Blog"
cd mysite

# Set up the database
pebble migrate

# Start the server
pebble serve
```

Visit `http://localhost:3000/admin/setup` to create your admin account.

## CLI Commands

### init

Create a new site:

```bash
pebble init [PATH] --name "Site Name"
```

This creates:
- `pebble.toml` - Configuration file
- `data/` - Database and media storage
- `themes/` - Custom themes (optional)

### serve

Start the development server:

```bash
pebble serve                     # Default: 127.0.0.1:3000
pebble serve -H 0.0.0.0 -p 8080  # Custom host/port
```

### deploy

Start the production server (defaults to 0.0.0.0:8080):

```bash
pebble deploy
pebble deploy -p 3000
```

### build

Generate a static version of your site:

```bash
pebble build --output ./dist --base-url "https://example.com"
```

### migrate

Run database migrations:

```bash
pebble migrate
```

### user

Manage users from the command line:

```bash
pebble user add --username alice --email alice@example.com --role admin
pebble user list
pebble user passwd alice
pebble user remove alice
```

Roles: `admin`, `author`, `viewer`

### backup

Manage system backups:

```bash
# Create a backup (defaults to ./backups)
pebble backup create

# List available backups
pebble backup list

# Restore from a backup file
pebble backup restore backups/backup_20230101.tar.gz
```

### import / export

Migrate content between Pebble instances:

```bash
# Export content to JSON
pebble export --output ./export --include-drafts --include-media

# Import content from export directory
pebble import ./export --overwrite
```

## Configuration

Edit `pebble.toml`:

```toml
[site]
title = "My Blog"
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

[media]
upload_dir = "./data/media"
max_upload_size = "10MB"

[theme]
name = "default"

[auth]
session_lifetime = "7d"
```

## Themes

Pebble includes three built-in themes, each with a distinct visual style. All themes support both light and dark modes.

### Available Themes

#### Default

A modern, clean theme with soft colors and rounded corners. Uses a blue primary color and sans-serif typography. Best for personal blogs and general-purpose sites.

- Rounded corners on cards and buttons
- Subtle shadows for depth
- Blue primary color (#3b82f6)
- Sans-serif font stack

#### Minimal

A stark, monochromatic theme focused on typography and whitespace. Removes visual clutter to emphasize content.

- No rounded corners (sharp edges)
- No shadows
- Monochromatic color scheme
- Uppercase headings with letter-spacing
- Maximum readability with increased line-height

#### Magazine

A bold, editorial theme inspired by newspaper and magazine layouts. Uses serif typography and strong visual hierarchy.

- Serif font family (Georgia)
- Red primary color (#b91c1c)
- Double-line borders on header/footer
- Large, bold headlines
- Warm background tones

### Switching Themes

To change your theme, edit `pebble.toml`:

```toml
[theme]
name = "magazine"  # Options: "default", "minimal", "magazine"
```

Restart the server after changing themes.

### Custom Theme Options

You can customize colors, fonts, and other visual properties without creating a full custom theme. Add a `[theme.custom]` section to your `pebble.toml`:

```toml
[theme]
name = "default"

[theme.custom]
# Colors (use any valid CSS color value)
primary_color = "#8b5cf6"        # Primary brand color
primary_color_hover = "#7c3aed"  # Primary color on hover
accent_color = "#f59e0b"         # Accent color for highlights
background_color = "#fafafa"     # Main background
background_secondary = "#f3f4f6" # Secondary background (cards, sections)
text_color = "#111827"           # Main text color
text_muted = "#6b7280"           # Muted/secondary text
border_color = "#e5e7eb"         # Border color
link_color = "#2563eb"           # Link color (defaults to primary)

# Typography
font_family = "Inter, system-ui, sans-serif"       # Body text font
heading_font_family = "Playfair Display, Georgia"  # Heading font
font_size = "18px"                                 # Base font size
line_height = 1.7                                  # Line height for body text

# Layout
border_radius = "0.75rem"  # Border radius (use "0" for sharp corners)
```

All custom options are optional. Only specify the values you want to change; unspecified values will use the theme's defaults.

### Examples

**Purple theme with sharp corners:**

```toml
[theme]
name = "default"

[theme.custom]
primary_color = "#7c3aed"
primary_color_hover = "#6d28d9"
accent_color = "#ec4899"
border_radius = "0"
```

**Minimal theme with custom serif font:**

```toml
[theme]
name = "minimal"

[theme.custom]
font_family = "Merriweather, Georgia, serif"
heading_font_family = "Merriweather, Georgia, serif"
line_height = 1.8
```

**Magazine theme with blue instead of red:**

```toml
[theme]
name = "magazine"

[theme.custom]
primary_color = "#1d4ed8"
primary_color_hover = "#1e40af"
```

### Dark Mode

All themes support automatic dark mode based on the user's system preference. Users can also toggle between light and dark modes using the theme toggle button in the site header.

Custom colors defined in `[theme.custom]` apply to the light mode. Dark mode colors are automatically adjusted by the theme.

## Homepage Layout

Customize the homepage layout and design through the `[homepage]` section in `pebble.toml`.

### Basic Configuration

```toml
[homepage]
show_hero = true       # Show/hide the hero section
show_pages = true      # Show/hide the pages section
show_posts = true      # Show/hide the recent posts section
```

### Hero Section Options

The hero section is the main banner at the top of your homepage.

```toml
[homepage]
hero_layout = "centered"    # "centered", "left", or "banner"
hero_height = "medium"      # "small", "medium", "large", or "full"
hero_text_align = "center"  # "left", "center", or "right"
hero_image = "/media/hero-bg.jpg"  # Optional background image
```

**Hero Layouts:**

- `centered` - Content centered with gradient background (default)
- `left` - Content aligned left, minimal styling
- `banner` - Full-width banner that extends beyond the container

**Hero Heights:**

- `small` - Compact padding
- `medium` - Standard padding (default)
- `large` - Tall hero (50vh minimum)
- `full` - Near full-screen hero (80vh minimum)

### Posts Section Options

Control how recent posts are displayed on the homepage.

```toml
[homepage]
posts_layout = "grid"   # "grid", "list", or "compact"
posts_columns = 2       # Number of columns (1, 2, or 3) for grid layout
```

**Posts Layouts:**

- `grid` - Card grid layout (default)
- `list` - Single column list with full excerpts
- `compact` - Dense list without excerpts

### Pages Section Options

Control how pages are displayed on the homepage.

```toml
[homepage]
pages_layout = "grid"   # "grid", "list", or "compact"
```

**Pages Layouts:**

- `grid` - Card grid layout (default)
- `list` - Vertical list
- `compact` - Inline button-style links

### Complete Example

```toml
[homepage]
# Hero configuration
show_hero = true
hero_layout = "centered"
hero_height = "large"
hero_text_align = "center"
hero_image = "/media/mountains.jpg"

# Sections visibility
show_pages = true
show_posts = true

# Posts display
posts_layout = "grid"
posts_columns = 2

# Pages display
pages_layout = "compact"
```

### Homepage Content (Admin UI)

In addition to layout options in `pebble.toml`, you can customize homepage text content through the admin panel at `/admin/settings`:

- **Homepage Title** - Custom hero title (defaults to site title)
- **Homepage Subtitle** - Custom tagline (defaults to site description)
- **Custom Content** - HTML content below the subtitle (for CTAs, buttons, etc.)

## Content Types

### Posts

Blog posts with:
- Markdown body with syntax highlighting
- Tags for categorization
- Auto-generated excerpts
- Draft/Published/Archived status

### Pages

Static pages (About, Contact, etc.) without tags or date-based ordering.

## Writing Content

The editor supports full Markdown with live preview:

```markdown
# Heading

Regular paragraph with **bold** and *italic* text.

## Code Blocks

```rust
fn main() {
    println!("Syntax highlighted!");
}
```

## Lists

- Item one
- Item two

## Links and Images

[Link text](https://example.com)
![Alt text](/media/image.jpg)
```

## Admin Interface

### Dashboard (`/admin`)

Overview of your site with quick stats and recent posts.

### Posts (`/admin/posts`)

- List all posts
- Create/edit with live markdown preview
- Manage tags inline
- Set status (draft, published, archived)

### Pages (`/admin/pages`)

Same as posts, but for static pages.

### Media (`/admin/media`)

Upload and manage images and files. Use in content as:

```markdown
![Description](/media/filename.jpg)
```

### Tags (`/admin/tags`)

View all tags with post counts. Tags are auto-created when you add them to posts.

### Settings (`/admin/settings`)

View current configuration (edit `pebble.toml` to change).

## Public Routes

| Route | Description |
|-------|-------------|
| `/` | Homepage with latest posts |
| `/posts` | Paginated post listing |
| `/posts/:slug` | Single post |
| `/pages/:slug` | Single page |
| `/tags` | All tags |
| `/tags/:slug` | Posts by tag |
| `/search?q=` | Search results |
| `/feed.xml` | RSS feed |

## Deployment

### Systemd

```ini
[Unit]
Description=Pebble CMS
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/var/www/mysite
# Use 'deploy' for production defaults (0.0.0.0:8080)
ExecStart=/usr/local/bin/pebble deploy
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

### Reverse Proxy (Caddy)

```
example.com {
    reverse_proxy localhost:8080
}
```

### Reverse Proxy (Nginx)

```nginx
server {
    listen 80;
    server_name example.com;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Backup & Recovery

Pebble includes built-in tools for backup and recovery.

**Create a backup:**
```bash
pebble backup create
```
This archives the database, configuration, and media files into the `backups/` directory.

**Restore a backup:**
```bash
pebble backup restore ./backups/pebble_backup_2024-01-01_12-00-00.tar.gz
```
*Warning: This will overwrite current data.*

## Search

Full-text search is built-in using SQLite FTS5. It searches:
- Post/page titles
- Body content
- Tags

Access via `/search` or the search box in navigation.


