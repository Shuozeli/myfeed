# Atom Feed Output

## Problem

Telegram is the only notification channel. Users who prefer RSS/Atom
feed readers have no way to consume myfeed output.

## Solution

After each crawl cycle, regenerate a static `feed.xml` (Atom format)
from the most recent items in `feed_items`. Serve it however you want
(nginx, python http.server, etc.).

## Design

### When to generate

After the full crawl loop (all sites complete), not after each site.
One feed with all sites mixed, sorted by time.

### Feed content

- Feed title: `myfeed`
- Feed ID: configurable or derived from output path
- Each entry:
  - `<title>`: `[site] item title`
  - `<link>`: item URL
  - `<id>`: `site:external_id`
  - `<published>`: `created_at` from DB
  - `<content>`: preview text
  - `<category>`: site name
- Last 100 items (configurable)

### Format

Atom 1.0. Cleaner spec than RSS 2.0 -- proper timestamps, unique IDs,
content element. Supported by every feed reader.

### Implementation

New file: `src/feed.rs`
- `generate_atom(items: &[FeedItem]) -> String` -- pure function,
  builds XML with `format!`. No XML crate needed.

Scheduler change: after the crawl loop, if `FEED_OUTPUT_PATH` is set,
query recent items and write `feed.xml`.

### Config

```
FEED_OUTPUT_PATH=feed.xml   # optional, empty/unset = disabled
FEED_ITEM_COUNT=100         # optional, default 100
```

Both optional -- Atom feed is opt-in. No change to existing behavior
if not configured.

### No new dependencies

Atom XML is simple enough to generate with string formatting:

```xml
<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>myfeed</title>
  <updated>2026-03-23T01:00:00Z</updated>
  <entry>
    <title>[reddit] Post title</title>
    <link href="https://..." />
    <id>reddit:abc123</id>
    <published>2026-03-23T00:30:00Z</published>
    <content type="text">Preview text...</content>
    <category term="reddit" />
  </entry>
</feed>
```
