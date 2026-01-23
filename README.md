# Pebble CMS

A lightweight, high-performance personal CMS built with Rust. Designed for simplicity, speed, and reliability.

## Features

- **Blazing Fast**: Built on the Axum web framework and SQLite.
- **Dual Mode**: Run as a dynamic server or generate a static site.
- **Markdown First**: Write content in standard Markdown with syntax highlighting.
- **Built-in Admin**: Simple, clean interface for managing posts, pages, tags, and media.
- **Zero Config**: Single binary deployment with automated database migrations.
- **Secure**: Dedicated production mode (`pebble deploy`) restricts admin access and hardens security.

## Installation

To install directly from source:

```bash
cargo install --path .
```

## Quick Start

1. **Initialize a site**
   ```bash
   pebble init my-blog
   cd my-blog
   ```

2. **Create an admin user**
   ```bash
   pebble user create --username admin --email admin@example.com --role admin
   ```

3. **Start development server**
   ```bash
   pebble serve
   ```
   Access the site at `http://localhost:3000` and the admin panel at `http://localhost:3000/admin`.

## Core Commands

- `pebble serve`: Start the development server (dynamic, includes admin).
- `pebble deploy`: Start the production server (read-only content, no admin access).
- `pebble build`: Generate a static HTML version of the site to `public/`.
- `pebble user`: Manage standard and admin users.
- `pebble import / export`: Migrate content via ZIP archives.
- `pebble backup`: Create database snapshots.

## Configuration

Configuration is managed via `pebble.toml` in your site directory.

```toml
[site]
title = "My Pebble Site"
url = "http://localhost:3000"

[content]
posts_per_page = 10
auto_excerpt = true
```

## License

MIT
