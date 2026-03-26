# Design

## Problem

You follow content across multiple social media platforms (Reddit, Zhihu, Weibo, X).
Checking each app multiple times a day is a context switch cost that adds up. APIs
exist for some platforms but are unreliable -- rate-limited, paywalled, deprecated,
or unable to access your personal logged-in feed.

## Solution

A self-hosted daemon that:
1. Drives a real Chrome browser with your logged-in sessions
2. Extracts new posts using declarative YAML recipes
3. Deduplicates via SQLite
4. Delivers new content to a single Telegram chat

The user logs in once per site. The daemon does the rest.

## Non-Goals

- **Not a public service.** Single-user, single-machine tool.
- **Not a search engine.** No full-text indexing. Just dedup-and-forward.
- **Not a recommendation system.** Shows everything from your feed as-is.
- **Not real-time.** Polling every 30 minutes is sufficient for the use case.

## Key Design Choices

### Browser Automation Over APIs

APIs are site-specific, require registration, and can't see your logged-in feed.
Browser automation via pwright is universal: if you can see it in Chrome, myfeed
can extract it. This also makes myfeed a practical showcase of pwright's recipe
system for real-world automation.

### YAML Recipes Over Hardcoded Crawlers

Site-specific DOM knowledge changes frequently. Keeping it in YAML means:
- No recompilation when a site redesigns
- Clear separation between "what to extract" (recipe) and "how to process" (Rust)
- Users can add new sites without touching Rust code
- Recipes are testable standalone via `pwright script run`

### SQLite Over Postgres

This is a personal single-user tool. SQLite eliminates the need for an external
database server. diesel provides type-safe queries and migrations. The entire
state is one file, easy to back up or reset.

### Telegram for Delivery

Simple Bot API (one HTTP POST per message), works on mobile, supports rich
HTML formatting, free for personal use, no infrastructure to manage.

### Sync Diesel in Async Runtime

Diesel is synchronous. Rather than fight this with async wrappers, we use
`tokio::task::spawn_blocking` for DB calls. The single-writer workload
(one crawl at a time per site) makes the Mutex-wrapped connection sufficient.

## Recipe Contract

Every feed recipe must output a JSON array in the `items` key:

```json
[{"id": "...", "title": "...", "url": "...", "preview": "..."}]
```

- `id`: Unique key for deduplication (site + id must be globally unique)
- `title`: Display title (first 80 chars of content for text-only sites)
- `url`: Link to the original post
- `preview`: First 200 chars of content (optional, shown in Telegram)

## Phased Rollout

| Phase | Scope | Status |
|-------|-------|--------|
| Dark launch | Reddit + HN, manual `myfeed once` | Done |
| Multi-site | Initial 7 sites verified against live Chrome | Done |
| Snapshots | Protobuf-typed crawl snapshots + Atom feed | Done |
| Agent digest | `myfeed dump` with tiered sharding + prompt templates | Done |
| Polish | Keyword filtering, digest mode, dedup windows | Done |
| Scale-out | 26 public + 6 private sites (32 total) | Done |

## Event Log

Every crawl cycle logs structured events to the `event_log` table:

| Event | When | Details |
|-------|------|---------|
| `crawl_start` | Before running a recipe | `{}` |
| `crawl_complete` | After processing all items | `{items_found, new_items}` |
| `crawl_error` | On any failure | `{error}` |

Query via `myfeed events -l 20` for debugging.
