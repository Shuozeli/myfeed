# Tasks

## Done

- [x] Project scaffold (Cargo.toml, src/, recipes/, docs/, CI, pre-commit)
- [x] Core modules: config, db (SQLite/diesel), telegram, crawler, scheduler
- [x] CLI: run, once, login, list, snapshots, snapshot, dump, events
- [x] 26 public sites + 6 private sites (32 total)
- [x] All recipes fetch post content (not just titles)
- [x] Protobuf schema with per-site oneof payloads
- [x] Crawl snapshots stored as JSON in SQLite
- [x] Action recipes: follow-linkedin, follow-x, join-reddit
- [x] Code review: fix tab leak, race condition, error swallowing
- [x] Atomic dedup via INSERT OR IGNORE
- [x] db_blocking helper to reduce spawn_blocking boilerplate
- [x] Recipe path resolves via RECIPES_DIR, checks private/ first
- [x] Design comparison doc (vs Page Agent, browser-use, PinchTab, OpenClaw)
- [x] Atom feed output (static feed.xml, opt-in via FEED_OUTPUT_PATH)
- [x] Agent-readable dump with tiered sharding (compact index + detail by ID)
- [x] Prompt templates for agent digests (daily-digest, trending-topics, tech-radar)
- [x] Keyword filtering (FILTER_KEYWORDS, items still saved to DB)
- [x] Digest mode (DIGEST_MODE, batch Telegram messages per site)
- [x] Per-site 2-minute timeout with 3 retries
- [x] Stale recipe detection (3 consecutive empty crawls -> Telegram warning)
- [x] Event log retention (30 days, cleanup at cycle start)
- [x] Telegram message queue with rate limiting (1 msg/sec, 429 backoff)
- [x] Site wishlist: all P0/P1/P2 sites explored, recipes written
- [x] 24-hour dark launch with 30 sites

## Pending

- [ ] Integration tests with Lightpanda browser (CI job)

## Completed (this session)

- [x] Improve eastmoney-feed.yaml: Added batch article content fetching (top 20)
- [x] Improve finviz-feed.yaml: Added batch article content fetching with meta description fallback
- [x] Improve seekingalpha-feed.yaml: Added batch article content fetching
- [x] Improve futunn-feed.yaml: Added batch article content fetching
- [x] Improve substack-feed.yaml: Added batch article content fetching (top 10)

### Code Review Fixes
- [x] Fixed TOCTOU race condition in dedup: new `insert_item_is_new()` method atomically checks and inserts
- [x] Parallelized site crawling: uses `futures::future::join_all` with semaphore (5 concurrent)
- [x] Panic on startup: `FeedDb::new` and `TelegramBot::new` now return `Result`, Telegram falls back to StdoutNotifier on error
