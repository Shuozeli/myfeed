# Code Quality Findings

Audit date: 2026-03-23

## Fixed

| Issue | Location | Fix |
|-------|----------|-----|
| Crawl loop duplicated in Once vs scheduler | main.rs, scheduler.rs | Once now calls `scheduler::crawl_site()` |
| BrowserConfig construction duplicated | main.rs, scheduler.rs | Moved to `Config::browser_config()` |
| `db.insert()` error silently discarded | main.rs | Removed -- Once uses crawl_site() |
| `serde_json::to_string().unwrap_or_default()` | db.rs | Changed to `.expect()` (infallible) |
| `serde_json::to_value().unwrap_or_default()` | crawler.rs | Changed to `.expect()` (infallible) |
| `db.exists()` error masked as false | main.rs | Removed -- uses `insert_if_new()` |
| Tab leak on recipe error | scheduler.rs | Extracted `crawl_with_page()`, tab always closed |
| Race condition exists()+insert() | db.rs | Replaced with atomic `insert_if_new()` (INSERT OR IGNORE) |
| Error swallowed in error handler | scheduler.rs | `let _ =` replaced with `if let Err` + warn |
| No HTML escaping in Telegram | telegram.rs | Added `escape_html()` |
| Empty URL renders broken link | telegram.rs | Conditional `<a>` tag |
| pwright-cdp unused dependency | Cargo.toml | Removed |
| chrono serde feature unused | Cargo.toml | Removed |
| diesel chrono feature unused | Cargo.toml | Removed |
| Command::Once missing event logging | main.rs | Reuses `crawl_site()` |
| No timeout on Telegram HTTP client | telegram.rs | 30s timeout on reqwest Client |
| spawn_blocking boilerplate | scheduler.rs | Extracted `db_blocking()` helper |
| Comments restating code | scheduler.rs | Removed |
| Snapshot parse error not logged | main.rs | Added `tracing::warn` |
| Unknown site proto oneof silent | crawler.rs | Added `tracing::warn` |
| Recipe path relative to CWD | crawler.rs | Checks RECIPES_DIR, falls back to exe-relative |

## Open

### Stringly-typed site names
- **Location:** config.rs, scheduler.rs, crawler.rs, db.rs, main.rs
- **Problem:** Site names are unvalidated strings. Typo in .env silently skipped.
- **Fix:** Define `Site` enum with `FromStr`. Low priority -- current behavior (warn + skip) is acceptable.

### Login URLs hardcoded in match arm
- **Location:** main.rs
- **Problem:** Adding a site requires modifying the match statement.
- **Fix:** Move login URLs to the `Site` enum or a config file. Low priority.

### `expect()` in CLI command handlers
- **Location:** main.rs (Login, Events, List, Snapshots)
- **Problem:** Panics instead of user-friendly error messages.
- **Fix:** Use `Result` propagation with `anyhow` or manual error handling. Low priority.

### `send_message` is pub but only used internally
- **Location:** telegram.rs
- **Fix:** Could be `pub(crate)`. Cosmetic.

### Event log grows unbounded
- **Location:** db.rs
- **Fix:** Add retention cleanup (delete >30 days). Low priority.

### Explore/feed recipe YAML duplication
- **Problem:** All recipes share identical structure, only selectors differ.
- **Fix:** Inherent to YAML recipes. Would need template/include in pwright-script.

### `read_line` result discarded in Login
- **Location:** main.rs
- **Fix:** Low priority for interactive command.

### Hand-rolled error types
- **Location:** crawler.rs (CrawlError), telegram.rs (TelegramError)
- **Fix:** Add `thiserror` derive. Saves ~30 lines. Low priority.
