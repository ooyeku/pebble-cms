# Pebble

A lightweight, single-binary CMS built with Rust. Run it as a dynamic server, or generate a fully static site. No runtime dependencies, no external database -- just one binary and a SQLite file.

## Highlights

- **Single binary** -- no runtime, no containers, no interpreters. One `pebble` binary does everything.
- **SQLite storage** -- all content lives in a single database file alongside your config.
- **Dual mode** -- serve dynamically with a full admin interface, or generate static HTML for zero-cost hosting.
- **Markdown with shortcodes** -- write in Markdown with syntax highlighting, embed media using shortcodes like `[image]`, `[video]`, `[audio]`, and `[gallery]`.
- **Built-in image pipeline** -- uploads are automatically optimized, converted to WebP, and thumbnailed.
- **Multi-site registry** -- manage multiple sites from one machine with automatic port assignment.
- **Five built-in themes** -- default, minimal, magazine, brutalist, and neon. Every theme supports custom color and typography overrides.

## Quick Start

```bash
cargo install --path .

pebble init my-blog
cd my-blog
pebble serve
```

Open `http://localhost:3000/admin` to create your first admin account and start writing.

## Commands

| Command | Description |
|---------|-------------|
| `pebble init [PATH]` | Create a new site |
| `pebble serve` | Start the development server (`127.0.0.1:3000`) |
| `pebble deploy` | Start the production server (`0.0.0.0:8080`) |
| `pebble build` | Generate a static site |
| `pebble export` | Export content to JSON files |
| `pebble import` | Import content from JSON files |
| `pebble backup create` | Create a backup archive |
| `pebble backup restore <FILE>` | Restore from a backup |
| `pebble user add` | Add a user account |
| `pebble migrate` | Run database migrations |
| `pebble config list` | View global settings |
| `pebble registry init <NAME>` | Create a registry-managed site |
| `pebble registry serve <NAME>` | Start a registry site |

Run any command with `--help` for full usage details.

## Configuration

Each site is configured with a `pebble.toml` at the site root:

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

Themes can be customized in-place by adding a `[theme.custom]` section:

```toml
[theme.custom]
primary_color = "#e63946"
background_color = "#f1faee"
text_color = "#1d3557"
font_family = "Georgia, serif"
```

See [docs/usage.md](docs/usage.md) for the complete configuration reference.

## Content

Posts and pages are written in Markdown with support for tables, footnotes, strikethrough, task lists, and fenced code blocks with syntax highlighting.

Media can be embedded directly using shortcodes:

```markdown
[image src="photo.jpg" alt="A sunset over the mountains"]

[video src="demo.mp4" controls]

[gallery src="img1.jpg,img2.jpg,img3.jpg" columns="3"]
```

Content statuses: **draft**, **scheduled**, **published**, **archived**.

## Static Site Generation

```bash
pebble build --output ./public --base-url https://example.com
```

Produces a self-contained directory with HTML pages, paginated archives, tag pages, an RSS feed, a JSON Feed, a sitemap, a client-side search page, and all media files. Deploy it anywhere that serves static files.

## Multi-Site Registry

Manage multiple sites from a single machine. Sites are stored under `~/.pebble/registry/` and can be started, stopped, and monitored from any directory.

```bash
pebble registry init blog --title "My Blog"
pebble registry init docs --title "Documentation"
pebble registry serve blog       # auto-assigns port 3001
pebble registry serve docs       # auto-assigns port 3002
pebble registry list
pebble registry stop-all
```

## Global Configuration

Global defaults for new sites are stored in `~/.pebble/config.toml` and managed through the CLI:

```bash
pebble config list
pebble config set defaults.theme minimal
pebble config set defaults.posts_per_page 15
```

## User Management

```bash
pebble user add --username alice --email alice@example.com --role admin
pebble user list
pebble user passwd alice
pebble user remove alice
```

Roles: **admin** (full access), **author** (own content), **viewer** (read-only).

## Backups

```bash
pebble backup create
pebble backup list
pebble backup restore ./backups/pebble-backup-2025-01-15-120000.tar.gz
```

## Documentation

Full usage guide: [docs/usage.md](docs/usage.md)

## Built With

[Axum](https://github.com/tokio-rs/axum) | [SQLite](https://www.sqlite.org/) (via rusqlite) | [Tera](https://keats.github.io/tera/) templates | [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) | [syntect](https://github.com/trishume/syntect) syntax highlighting

## License

[MIT](LICENSE)
