# Tasks

## Done

- [x] Project scaffold (Cargo.toml, src/, recipes/, docs/, CI, pre-commit)
- [x] Core modules: config, db (SQLite/diesel), telegram, crawler, scheduler
- [x] CLI: run, once, login, list, snapshots, snapshot, events
- [x] 7 sites: reddit, zhihu, xueqiu, x, linkedin, hackernews, 1point3acres
- [x] Feed + exploration recipes for all sites
- [x] All recipes fetch post content (not just titles)
- [x] Protobuf schema with per-site oneof payloads
- [x] Crawl snapshots stored as JSON in SQLite
- [x] Action recipes: follow-linkedin, follow-x, join-reddit
- [x] Code review: fix tab leak, race condition, error swallowing
- [x] Atomic dedup via INSERT OR IGNORE
- [x] db_blocking helper to reduce spawn_blocking boilerplate
- [x] Recipe path resolves via RECIPES_DIR or relative to executable
- [x] Design comparison doc (vs Page Agent, browser-use, PinchTab, OpenClaw)
- [x] Documentation: README, architecture, design, codelabs, CLAUDE.md

## Pending

- [ ] Test end-to-end with real Telegram bot
- [ ] Validate zhihu recipe against live site (needs login)
- [ ] Add keyword filtering to crawler
- [ ] Add digest mode (batch Telegram messages)
- [ ] Add retry logic for transient CDP connection failures
- [ ] Add event_log retention (cleanup old entries)
