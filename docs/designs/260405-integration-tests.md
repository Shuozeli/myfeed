# Integration Test Infrastructure Design

<!-- agent-updated: 2026-04-05T17:45:00Z -->

## Overview

Redesigned myfeed integration tests to use Lightpanda browser (lightweight, fast) instead of Chrome.

## Goals

1. **Fast CI** - Lightpanda starts in <1s vs Chrome's ~10s
2. **Self-contained** - No external browser dependency in CI
3. **Realistic testing** - Demo servers simulate actual feed sites
4. **Deterministic** - Known content, no flakiness from external sites

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   CI Integration Test                        │
│  ┌──────────────┐   ┌──────────────┐   ┌────────────────┐  │
│  │ Lightpanda   │◄─►│  myfeed CLI  │   │  CI Runner     │  │
│  │ Browser      │   │  (crawl)     │   │                │  │
│  │ (container)  │   │              │   │                │  │
│  └──────────────┘   └──────────────┘   └────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Components

### 1. Test Files

```
tests/
  cli.rs           - CLI integration tests (no browser needed)
  integration.rs   - Full integration tests with browser
```

### 2. Browser: Lightpanda

**Why Lightpanda:**
- Startup time: <1s (vs Chrome ~10s)
- Memory: ~50MB (vs Chrome ~300MB)
- Uses WebSocket directly for CDP
- Works well in containers

**Docker:**
```yaml
lightpanda:
  image: lightpanda/browser:nightly
  ports:
    - "9222:9222"  # WebSocket CDP
  shm_size: '256mb'
```

### 3. Integration Tests

Tests marked with `#[ignore]` that require a running browser:
- `test_crawl_simple_feed_with_browser`
- `test_crawl_hackernews_mock_with_browser`
- `test_crawl_output_formats`
- `test_crawl_compact_output`
- `test_crawl_save_to_db`
- `test_recipe_validate_with_browser`

Tests that run without browser:
- `test_crawl_requires_site`
- `test_crawl_unknown_site_graceful`
- `test_recipe_list_shows_recipes`

## Running Tests

```bash
# Unit tests only (no browser)
cargo test --lib

# CLI tests only (no browser)
cargo test --test cli

# Integration tests without browser
cargo test --test integration

# Full tests including browser-dependent (requires Lightpanda)
cargo test --test integration -- --ignored --nocapture
```

## Docker Compose

```bash
# Start Lightpanda only
docker compose -f docker-compose.test.yml up lightpanda

# Run unit tests
docker compose -f docker-compose.test.yml run unit-test

# Run integration tests (with Lightpanda)
docker compose -f docker-compose.test.yml --profile integration run integration-test
```

## CI Pipeline

```yaml
integration-test:
  steps:
    - name: Start Lightpanda
      run: docker run -d --name lightpanda -p 9222:9222 lightpanda/browser:nightly

    - name: Wait for Lightpanda
      run: for i in {1..30}; do curl -s http://localhost:9222 && break; sleep 1; done

    - name: Run integration tests
      env:
        CDP_ENDPOINT: ws://localhost:9222
      run: cargo test --test integration -- --ignored --nocapture
```

## Lightpanda vs Chrome

| Aspect | Chrome | Lightpanda |
|--------|--------|------------|
| Startup time | ~10s | <1s |
| Memory | ~300MB | ~50MB |
| CDP | HTTP+WS | WebSocket only |
| Container size | ~1GB | ~50MB |
| CI fit | Slower | Ideal |

## Files Created/Modified

- `docker-compose.test.yml` - Updated to use Lightpanda
- `Dockerfile` - Multi-stage build for testing
- `.dockerignore` - Exclude unnecessary files
- `tests/cli.rs` - CLI integration tests
- `tests/integration.rs` - Full integration tests with browser
- `.github/workflows/ci.yml` - Updated CI with Lightpanda job

## Recipe Tests

Test recipes for demo/mock sites:
```
recipes/test/
  simple-feed.yaml      - Minimal feed (id, title, url, preview)
  hackernews-mock.yaml  - HN-style (score, comments, age)
  reddit-mock.yaml      - Reddit-style (upvotes, author)
```
