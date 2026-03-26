# Code Quality Findings

Audit date: 2026-03-26

## Previously Fixed (from 2026-03-23 audit)

| Issue | Location | Fix |
|-------|----------|-----|
| Crawl loop duplicated in Once vs scheduler | main.rs, scheduler.rs | Once now calls `scheduler::crawl_site()` |
| BrowserConfig construction duplicated | main.rs, scheduler.rs | Moved to `Config::browser_config()` |
| `db.insert()` error silently discarded | main.rs | Removed -- Once uses crawl_site() |
| `serde_json::to_string().unwrap_or_default()` | db.rs | Changed to `.expect()` (infallible) |
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

---

## Previously Fixed (2026-03-23 incremental audit)

| Issue | Fix |
|-------|-----|
| `serde_json::to_value()` silently returns Null | Changed to `.expect()` (infallible) |
| `read_line` result fully discarded in Login | Added error check with `process::exit(1)` |
| Telegram drain rate comment contradicts log | Fixed sleep to 1s, updated comment |
| Login error message incomplete site list | Added all supported sites |
| `escape_xml`/`escape_html` near-identical | `escape_xml` now delegates to `escape_html` |
| `site.to_string()` repeated in scheduler | Allocated `owned_site` once at top |
| `json!()` for event log details | Defined `CrawlCompleteEvent`/`CrawlErrorEvent` typed structs |
| Atom entries missing `<updated>` | Added `<updated>` element |

---

## 2026-03-26 Audit Findings

### Category 1: Clippy Pedantic Issues (Severity: Low-Medium)

#### 1.1 `format!()` push_str pattern in feed.rs -- DONE
- **Location:** `src/feed.rs:15-32`
- **Problem:** 9 instances of `xml.push_str(&format!(...))` which allocates an intermediate String.
- **Fix:** Use `write!()` macro with `use std::fmt::Write` to write directly into the String.

#### 1.2 `map().unwrap_or()` instead of `map_or()` / `is_some_and()` -- DONE
- **Location:** `src/config.rs:59-62`, `src/feed.rs:5-8`
- **Problem:** Clippy recommends `is_some_and()` and `map_or()` for clarity and to avoid intermediate Option.
- **Fix:** Replace with idiomatic forms.

#### 1.3 Redundant closure in `int_field` -- DONE
- **Location:** `src/crawler.rs:142`
- **Problem:** `and_then(|v| v.as_i64())` is a redundant closure; `and_then(serde_json::Value::as_i64)` is clearer.
- **Fix:** Use method reference.

#### 1.4 Unchecked `as` casts may truncate/wrap -- DONE
- **Location:** `src/crawler.rs:142` (`i64 as i32`), `src/db.rs:152` (`u64 as i64`), `src/db.rs:302` (`usize as i32`), `src/main.rs:273` (`u64 as i64`), `src/scheduler.rs:262` (`usize as u32`)
- **Problem:** These casts can silently truncate or wrap. While practically unlikely with the data sizes involved, they are technically unsafe.
- **Fix:** Add explicit `#[allow(clippy::cast_possible_truncation)]` / `#[allow(clippy::cast_possible_wrap)]` annotations where the cast is intentional and safe given the domain constraints, or use `try_from` where feasible.

#### 1.5 Doc comments missing backticks around code identifiers -- DONE
- **Location:** `src/crawler.rs:12,62,147`, `src/db.rs:74,164,247`, `src/scheduler.rs:17,200`, `src/telegram.rs:46,98`
- **Problem:** Identifiers in doc comments should use backticks for clarity.
- **Fix:** Add backticks.

#### 1.6 `main()` too many lines (215 lines) -- DONE
- **Location:** `src/main.rs:92`
- **Problem:** The `main()` function is 215 lines, mostly due to the match arms for each CLI command.
- **Fix:** Extract the `Dump` command handler into a separate function, as it is the largest block (70+ lines).

### Category 2: Remaining `json!()` in non-test code (Severity: Low)

#### 2.1 `serde_json::json!({})` for crawl_start event -- SKIPPED
- **Location:** `src/scheduler.rs:154`
- **Problem:** Uses `json!({})` for an empty event detail. Per code standards, typed structs are preferred.
- **Reason for skip:** It is literally an empty object `{}`. Defining a zero-field struct adds ceremony for no type safety gain.

#### 2.2 `json!()` in Dump command output -- SKIPPED
- **Location:** `src/main.rs:248-305`
- **Problem:** Multiple `json!()` macros for CLI output formatting.
- **Reason for skip:** Output-only CLI code, no reader coupling, and defining DTOs for ad-hoc CLI output would add complexity without benefit.

### Category 3: Unnecessary Clones (Severity: Low)

#### 3.1 `items.clone()` for snapshot in crawl_with_page -- SKIPPED
- **Location:** `src/scheduler.rs:220`
- **Problem:** Full `items` Vec is cloned for the snapshot. Could be restructured to save snapshot first then iterate.
- **Reason for skip:** Small data (10-50 items). Restructuring adds complexity.

#### 3.2 `item.clone()` in dedup loop -- SKIPPED
- **Location:** `src/scheduler.rs:238,252`
- **Problem:** Items cloned for spawn_blocking closure boundary.
- **Reason for skip:** Required by the closure's `'static` bound. Would need batch operations to avoid.

### Category 4: Stringly-Typed APIs (Severity: Medium)

#### 4.1 Site names as unvalidated strings -- SKIPPED
- **Reason:** Cross-cutting refactor. Better as a dedicated design task.

### Category 5: Hand-Rolled Error Types (Severity: Low)

#### 5.1 CrawlError and TelegramError implement Display/Error manually -- SKIPPED
- **Reason:** `thiserror` dependency overhead not justified for ~30 lines of boilerplate.

---

## Summary of Actions (2026-03-26)

| Finding | Status |
|---------|--------|
| 1.1 format_push_string in feed.rs | DONE |
| 1.2 map().unwrap_or() patterns | DONE |
| 1.3 Redundant closure in int_field | DONE |
| 1.4 Unchecked as casts | DONE |
| 1.5 Doc comment backticks | DONE |
| 1.6 main() too many lines | DONE |
| 2.1 json!({}) for crawl_start | SKIPPED |
| 2.2 json!() in Dump command | SKIPPED |
| 3.1 items.clone() for snapshot | SKIPPED |
| 3.2 item.clone() in dedup loop | SKIPPED |
| 4.1 Stringly-typed site names | SKIPPED |
| 5.1 Hand-rolled error types | SKIPPED |
