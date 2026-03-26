# Architecture

## Overview

myfeed is a personal feed aggregator that uses browser automation (via pwright)
to crawl social media sites and deliver updates via Telegram and Atom feed.

```
                     +------------------+
                     |    Chrome CDP    |
                     |  (remote debug)  |
                     +--------+---------+
                              |
                     +--------+---------+
                     |  pwright-bridge  |
                     | (CDP connection) |
                     +--------+---------+
                              |
                     +--------+---------+
                     |  pwright-script  |
                     | (recipe executor)|
                     +--------+---------+
                              |
                     +--------+---------+
                     |    myfeed        |
                     |   crawler.rs     |
                     +--------+---------+
                              |
          +-------------------+-------------------+
          |                   |                   |
 +--------+--------+ +-------+--------+ +--------+--------+
 |   SQLite (diesel)| | Telegram Bot   | |   Atom Feed     |
 | dedup + events  | | notifications  | |   feed.xml      |
 | + snapshots     | +----------------+ +-----------------+
 +-----------------+
          |
 +--------+--------+
 | myfeed dump     |
 | (agent-readable |
 |  JSON output)   |
 +-----------------+
```

## Components

### Config (`config.rs`)
Reads all settings from environment variables. Fails immediately on any
missing required variable (fail-fast). Optional settings: `FEED_OUTPUT_PATH`,
`FEED_ITEM_COUNT`, `FILTER_KEYWORDS`, `DIGEST_MODE`, `DEDUP_WINDOW_HOURS`,
`DEDUP_OVERRIDES`.

### Crawler (`crawler.rs`)
- Loads pwright recipe YAML files
- Runs them against a browser Page via `pwright_script::executor::execute()`
- Parses output into `proto::FeedItem` with typed per-site payloads (oneof)
- Unknown sites produce a warning, not an error

### Scheduler (`scheduler.rs`)
- Runs the crawl loop on a configurable interval
- Retry: each site crawl retries up to 3 times with 5s delay
- Keyword filter: only matching items go to Telegram (all saved to DB)
- Digest mode: batch new items into one Telegram message per site
- Tab lifecycle safety: `crawl_with_page` ensures tab always closes on error
- After all sites: generates Atom feed if configured
- Event log retention: deletes entries older than 30 days at cycle start
- Uses `db_blocking()` helper for sync diesel calls in async context

### Database (`db.rs`)
- SQLite via diesel, Mutex-wrapped for thread safety
- `feed_items`: dedup via `INSERT OR IGNORE` on `(site, external_id)` unique constraint
- `crawl_snapshots`: full parsed JSON saved per site per crawl cycle
- `event_log`: time-series events for debugging
- All queries wrapped in transactions

### Feed (`feed.rs`)
- Generates Atom 1.0 XML from recent `feed_items`
- Written to disk after each crawl cycle (if `FEED_OUTPUT_PATH` is set)
- No dependencies -- built with `format!` and manual XML escaping

### Telegram (`telegram.rs`)
- Channel-based message queue: `TelegramSender` enqueues, `TelegramConsumer` drains
- Consumer rate: 1 message per second, with 429 `retry_after` backoff
- Formats items as HTML with proper escaping
- 30-second HTTP timeout, errors logged but don't block crawling

### Protobuf Schema (`proto/myfeed.proto`)
- `CrawlSnapshot`: site + timestamp + items array
- `FeedItem`: common fields (id, title, url, preview) + `oneof site_data`
- Per-site types: HackerNewsData, RedditData, ZhihuData, XData,
  LinkedInData, XueqiuData, OnePoint3AcresData

### Recipes (`recipes/`)
- `<site>-feed.yaml`: feed extraction (26 public + 6 private, 32 total)
- `explore/`: HTML structure discovery (run before writing feed recipes)
- `actions/`: follow/join recipes (LinkedIn, X, Reddit)

### Prompt Templates (`prompts/`)
- Pre-written prompts for AI agents to produce digests
- daily-digest, trending-topics, tech-radar

## Data Flow

1. Scheduler timer fires (default 30min)
2. Cleanup event log entries older than 30 days
3. For each enabled site (retry up to 3 times, 5s delay):
   a. Open a new Chrome tab via CDP (`connect_http`)
   b. Run the site's recipe YAML
   c. Crawler parses output into typed `proto::FeedItem`
   d. Save full snapshot to `crawl_snapshots`
   e. Atomic dedup: `INSERT OR IGNORE` into `feed_items`
   f. Apply keyword filter (all items saved, only matching notified)
   g. Send to Telegram (digest or per-item, depending on config)
   h. Close the tab (always, even on error)
4. Generate Atom feed (if configured)
5. Log completion event, sleep until next cycle

## AI Agent Integration

Agents read feed data via `myfeed dump` (not embedded LLM calls):

```
Agent                          myfeed
  |                              |
  |-- dump --compact --hours 24 ->|  (index: ~10 tokens/item)
  |<-- JSON with id/title/url ---|
  |                              |
  |-- dump --ids 42,55,78 ------>|  (detail: full preview + raw_json)
  |<-- JSON with full content ---|
  |                              |
  |-- (apply prompt template) ---|
  |-- (produce digest) ---------|
```

See `docs/agent-digest-guide.md` for the full workflow.

## Design Decisions

- **Browser-only, no APIs**: Works with any website, including login-gated content.
- **Recipe-based extraction**: Site logic in YAML, not Rust. Zero code changes to add a site.
- **Login-once model**: Chrome profile preserves session cookies between runs.
- **SQLite + diesel**: Lightweight, no external DB server, single-file backup.
- **Protobuf schema**: Typed per-site payloads with oneof. JSON-serialized for SQLite storage.
- **Atom feed**: Static file output, serve however you want (nginx, etc.).
- **Agent-readable, not agent-embedded**: No LLM calls in Rust. Agents read via `dump` command.
