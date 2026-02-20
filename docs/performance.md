# Performance Baseline

This document provides a methodology for benchmarking Pebble CMS and reference performance characteristics.

## Methodology

**Tool:** [oha](https://github.com/hatoo/oha) (or `wrk` / `hey` as alternatives)

**Reference dataset:**
- 100 published posts with tags and markdown content
- 20 published pages
- 50 tags
- 10 media files
- SQLite database ~2 MB

**Environment:** Single-core VPS, 1 GB RAM, SSD storage (representative of a $5/mo cloud instance).

## Benchmark Commands

```bash
# Homepage
oha -n 10000 -c 50 http://127.0.0.1:8080/

# Post page
oha -n 10000 -c 50 http://127.0.0.1:8080/posts/example-post

# Posts listing (paginated)
oha -n 10000 -c 50 http://127.0.0.1:8080/posts

# Search
oha -n 5000 -c 25 "http://127.0.0.1:8080/search?q=rust"

# API: list posts
oha -n 10000 -c 50 http://127.0.0.1:8080/api/v1/posts

# Health check
oha -n 10000 -c 50 http://127.0.0.1:8080/health
```

## Reference Performance Table

Results will vary by hardware. These are reference numbers on a 1-core, 1 GB RAM VPS:

| Endpoint | RPS | p50 (ms) | p99 (ms) | Notes |
|----------|-----|----------|----------|-------|
| `GET /health` | — | — | — | Baseline |
| `GET /` | — | — | — | Homepage with hero + posts |
| `GET /posts` | — | — | — | Paginated listing |
| `GET /posts/:slug` | — | — | — | Single post render |
| `GET /search?q=term` | — | — | — | FTS5 search |
| `GET /api/v1/posts` | — | — | — | JSON API |

> Run the benchmarks on your target hardware to fill in these values.

## Memory Profile

| Metric | Value |
|--------|-------|
| RSS at startup | ~10-15 MB |
| RSS under load (50 concurrent) | ~25-40 MB |
| Peak RSS | ~50 MB |

Pebble's memory footprint is intentionally small. SQLite operates within the configured `cache_size` (~64 MB page cache) and `mmap_size` (128 MB), but actual RSS depends on working set size.

## Binary Size

| Build | Size |
|-------|------|
| Debug | ~60-80 MB |
| Release (with LTO + strip) | ~8-12 MB |

Release profile settings in `Cargo.toml`:
```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

## Cold Start Time

Pebble starts in under 100ms on typical hardware. The startup sequence:

1. Load and validate config (~1ms)
2. Open SQLite connection pool (~5ms)
3. Run pending migrations (if any)
4. Rebuild FTS index (~10-50ms depending on content volume)
5. Bind TCP listener and serve

## SQLite Tuning Rationale

Pebble applies the following PRAGMA settings for optimal performance:

| Setting | Value | Rationale |
|---------|-------|-----------|
| `journal_mode` | WAL | Concurrent reads during writes |
| `synchronous` | NORMAL | Safe with WAL, faster than FULL |
| `busy_timeout` | 5000ms | Wait instead of failing on lock contention |
| `journal_size_limit` | 64 MB | Cap WAL file growth |
| `mmap_size` | 128 MB | Memory-mapped I/O for faster reads |
| `cache_size` | -65536 (64 MB) | In-memory page cache |
| `foreign_keys` | ON | Enforce referential integrity |

These defaults work well for personal blogs with up to thousands of posts. For high-traffic deployments, consider placing the database on SSD storage and increasing `cache_size`.
