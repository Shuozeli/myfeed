# Architecture

## Overview

myfeed is a personal feed aggregator that uses browser automation (via pwright)
to crawl social media sites and deliver updates via Telegram.

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
               +--------------+--------------+
               |                             |
      +--------+--------+          +--------+--------+
      |   SQLite (diesel)|         |  Telegram Bot   |
      |   dedup + events |         |  notifications  |
      +-----------------+          +-----------------+
```

## Components

### Config (`config.rs`)
Reads all settings from environment variables. Fails immediately on any
missing required variable (fail-fast, no defaults).

### Crawler (`crawler.rs`)
- Loads pwright recipe YAML files
- Runs them against a browser Page via `pwright_script::executor::execute()`
- Parses the recipe's output steps into `CrawledItem` structs
- Recipes output items in a standardized format: `{id, title, url, preview}`

### Scheduler (`scheduler.rs`)
- Runs the crawl loop on a configurable interval
- For each enabled site: connect to Chrome, run recipe, dedup, notify
- Logs events (start, complete, error) to the event_log table

### Database (`db.rs`)
- SQLite via diesel (sync, wrapped in Mutex for thread safety)
- `feed_items` table: dedup by `(site, external_id)` unique constraint
- `event_log` table: time-series events for debugging
- All queries run inside transactions

### Telegram (`telegram.rs`)
- Simple HTTP client wrapping the Telegram Bot sendMessage API
- Formats feed items as HTML messages with title, link, preview

### Recipes (`recipes/`)
- pwright YAML scripts that navigate to sites and extract posts
- `explore/` subdirectory for HTML structure discovery
- Feed recipes output items via JS `eval` steps that return JSON arrays
- Each item must have: `id` (dedup key), `title`, `url`, `preview`

## Data Flow

1. Scheduler timer fires
2. For each enabled site:
   a. Open a new Chrome tab via CDP
   b. Load and run the site's recipe YAML
   c. Recipe navigates to the site, waits for content, runs JS extraction
   d. Crawler parses recipe output into CrawledItem structs
   e. Check each item against SQLite (dedup by site + external_id)
   f. New items: insert into DB + send to Telegram
   g. Close the tab
3. Log completion event, sleep until next cycle

## Design Decisions

- **Browser-only, no APIs**: Every site is crawled via Chrome. This makes myfeed
  work with any website, including those behind login walls or without public APIs.
- **Recipe-based extraction**: Site-specific logic lives in YAML recipes, not Rust
  code. Adding a new site requires zero code changes.
- **Login-once model**: Chrome's user data directory preserves session cookies
  between runs. Users log in once, crawls reuse the session.
- **SQLite + diesel**: Lightweight, no external DB server needed. The Mutex-wrapped
  connection is fine for the single-writer workload (one crawl at a time).
- **Telegram for delivery**: Simple, works on mobile, supports rich formatting.
