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

Start the web server:

```bash
pebble serve                     # Default: 127.0.0.1:3000
pebble serve -H 0.0.0.0 -p 8080  # Custom host/port
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
ExecStart=/usr/local/bin/pebble serve -H 0.0.0.0
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

### Reverse Proxy (Caddy)

```
example.com {
    reverse_proxy localhost:3000
}
```

### Reverse Proxy (Nginx)

```nginx
server {
    listen 80;
    server_name example.com;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Backup

The entire site is contained in:
- `pebble.toml` - Configuration
- `data/pebble.db` - SQLite database (all content)
- `data/media/` - Uploaded files

Back these up regularly:

```bash
cp data/pebble.db data/pebble.db.backup
```

## Search

Full-text search is built-in using SQLite FTS5. It searches:
- Post/page titles
- Body content
- Tags

Access via `/search` or the search box in navigation.
