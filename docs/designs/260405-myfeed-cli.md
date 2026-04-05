# Design Doc: myfeed CLI

## Status

Proposed

## Context

myfeed is currently a daemon: it runs on a schedule, stores items in SQLite, and sends new ones to Telegram. This is the right model for personal feed aggregation, but it's not the right interface for:

- **Ad-hoc exploration** -- wanting to check one site right now without waiting for the next crawl cycle
- **AI agent tool use** -- agents calling myfeed as a CLI tool, passing site names and parameters
- **CI/testing** -- validating recipe changes in automation
- **One-off data extraction** -- "give me the top 10 HN stories right now"

AutoCLI demonstrates the CLI-as-product model well: `autocli twitter search "rust"` returns structured data, no config, no daemon. The adapter registry is the product.

We want the same for myfeed: `myfeed crawl hackernews` returns a JSON feed. The recipe library is the product.

## What vs AutoCLI

AutoCLI targets 55+ sites with public API/data extraction as its primary use case. It has AI adapter generation, adapter marketplace, and external CLI integration.

myfeed CLI targets the same problem space as its daemon: **personal feed aggregation**. The CLI is the ad-hoc interface to the same recipe engine. Key differences:

| | AutoCLI | myfeed CLI |
|--|---------|-----------|
| Primary use | Ad-hoc data extraction | Feed aggregation (daemon) + ad-hoc |
| Recipe format | `${{ }}` template expressions | Same as pwright (named scripts + `{{ }}` templates) |
| Browser layer | Daemon + Chrome Extension | pwright (CDP direct, attach-only) |
| Scheduling | No | Yes (via daemon) |
| Persistence | None | SQLite |
| Notification | No | Telegram (pluggable via Notifier trait) |
| gRPC API | No | Yes (inherits from pwright) |
| Agent adapter gen | Yes (`generate --ai`) | No (recipes are simple enough to write manually) |
| Desktop app control | Yes | No |
| External CLI tools | Yes | No |

## What the CLI Does

The CLI wraps the existing recipe engine (pwright-script) and exposes high-level feed commands. It does NOT replace the daemon -- the daemon is still the way to run scheduled aggregation with Telegram delivery.

```
myfeed --help
pwright [command]

Commands:
  crawl      Crawl one or more sites and output items as JSON
  list       List recent items from the local database
  recipe     Manage recipes (list, validate, generate)
  run        Start the daemon (existing)
  once       Run one crawl cycle and exit (existing)
  login      Open browser for manual login (existing)
  dump       Dump feed items as JSON for agent consumption (existing)
  events     Show event log (existing)

Options:
  --format   Output format: json, jsonl, table (default: json)
  --site     Filter by site name
  --hours    Time window for list/dump commands
```

## `myfeed crawl` Design

```
myfeed crawl [sites...] [flags]

Flags:
  --site <name>             Site to crawl (repeatable, or comma-separated)
  --param <key=val>         Recipe parameter (repeatable)
  --limit <n>               Max items to return (default: all)
  --dedup                   Deduplicate against local DB before output
  --format <json|jsonl>     Output format (default: json)
  --compact                 Omit previews, just id/title/url
  --save                    Save crawl snapshot to local DB
  --notify                  Send new items to Telegram (daemon must run)
```

**Behavior:**
- Resolves site names to recipe files (same lookup as daemon: `recipes/<site>-feed.yaml`)
- Runs each recipe via pwright-script executor
- Outputs JSON array of `{id, title, url, preview, site, score, comments, ...}`
- With `--save`: also persists to SQLite (so `--dedup` works on next run)
- With `--notify`: sends new (non-dup) items to Telegram
- `--dedup` requires `--save` first (or a prior daemon run populated the DB)

**Output examples:**

```bash
# Crawl one site, compact output
myfeed crawl hackernews --compact
[
  {"id":"424242","title":"Rust in production","url":"https://...","site":"hackernews"},
  {"id":"424241","title":"Show HN: We put a whole company in a single HTML file","url":"https://...","site":"hackernews"}
]

# Crawl multiple sites, full output
myfeed crawl hackernews,reddit --limit 5 --format json
[
  {"id":"424242","title":"Rust in production","url":"https://...","site":"hackernews","score":312,"comments":89,"preview":"..."},
  {"id":"abc123","title":"I ship's a Rust project and lived to tell the tale","url":"https://...","site":"reddit","subreddit":"r/rust","preview":"..."}
]

# Crawl with recipe params
myfeed crawl hackernews --param max_stories=10
```

**Exit codes:**
- 0: success (even if 0 items found)
- 1: recipe not found
- 2: crawl failed (timeout, CDP error)

## `myfeed recipe` Subcommands

```
myfeed recipe list           # List all available recipes
myfeed recipe list --site    # Filter by site name
myfeed recipe validate <site> # Validate recipe YAML + dry-run in headless Chrome
myfeed recipe test <site>    # Run recipe and print raw output
myfeed recipe edit <site>    # Open recipe in $EDITOR
```

`recipe validate` is new -- it does a dry-run with a test URL (or the recipe's default URL) to catch selector breakage before the daemon runs.

## Architecture

```
myfeed-cli (new crate)
├── src/main.rs              # CLI entry, clap argument parsing
├── src/crawl.rs             # `crawl` command implementation
├── src/output.rs            # JSON/JSONL/table formatting
└── src/recipe.rs            # `recipe` subcommands

myfeed (existing)
├── src/scheduler.rs          # Daemon loop (unchanged)
├── src/crawler.rs           # Recipe runner (used by both daemon and CLI)
├── src/db.rs                # SQLite (used by both)
├── src/notifier.rs          # Notifier trait (used by both)
└── recipes/                 # Recipe files (same format as pwright)
```

The CLI and daemon share the same recipe runner (`crawler.rs`), same DB, same notifier. The CLI just exposes a different interface -- ad-hoc rather than scheduled.

**New dependency:** `myfeed-cli` → `myfeed` crate (not pwright-script directly), so it inherits the recipe engine, DB, and notifier.

**No new crate needed** -- the CLI is a separate binary in the same Cargo workspace:

```toml
# Cargo.toml
[[bin]]
name = "myfeed"
path = "src/main.rs"

[[bin]]
name = "myfeed-cli"
path = "cli/src/main.rs"
```

Actually, `src/main.rs` is already the CLI with `run`, `once`, `login`, etc. Adding `crawl` as a subcommand to the existing binary is simpler than a separate binary. The daemon is started via `myfeed run`, not a separate binary.

So: add `crawl` as a new `Command` variant to the existing `main.rs`. No new crate needed.

## Data Flow for `myfeed crawl`

```
myfeed crawl reddit
  → resolve "reddit" → recipes/reddit-feed.yaml
  → connect to Chrome via CDP_ENDPOINT (env var)
  → pwright-script executor runs recipe
  → parse outputs into proto::FeedItem
  → format as JSON/JSONL/table
  → print to stdout
  → (optional: --save) insert into SQLite
  → (optional: --notify) send via Notifier trait
```

## Persistence Behavior

The CLI can read and write the local SQLite DB (`myfeed.db`). This means:

- `--dedup` works against data from prior `myfeed run` daemon cycles
- `--save` from CLI populates the DB for future `--dedup` runs
- The DB is shared between daemon and CLI

**Conflict:** If daemon is running and CLI writes to the same DB simultaneously, SQLite's locking handles it (writes are serialized). The CLI just needs to use `ON CONFLICT IGNORE` (same dedup logic as daemon).

## Key Design Decisions

### Reuse recipe format (not AutoCLI's pipeline format)

AutoCLI uses `${{ }}` template expressions with filters and pipes. pwright (and myfeed) use `{{ }}` Jinja-style templates with named scripts. 

We stick with pwright's format because:
1. It already works and recipes exist in this format
2. Named scripts in a registry are cleaner for complex extraction JS
3. AutoCLI's template expressions are more powerful but the tradeoff is readability

### `--notify` requires daemon for rate limiting

Telegram's rate limit (1 msg/sec) is enforced by the daemon's `TelegramConsumer`. If `myfeed crawl --notify` is run standalone, it bypasses rate limiting and could hit 429s.

Options:
1. Skip `--notify` for CLI entirely (only daemon does notifications)
2. Have CLI spawn a temporary consumer that drains at 1 msg/sec
3. Document that `--notify` is best-effost and may hit rate limits

Option 2 is correct -- the CLI already has `create_telegram_channel()` and can spawn the consumer. It's how `myfeed once` works.

### Recipe validation is a dry-run

`myfeed recipe validate <site>`:
1. Load recipe YAML
2. Connect to Chrome
3. Navigate to test URL
4. Run extraction JS
5. Print parsed items
6. Close Chrome

This catches selector breakage without modifying any state.

## Comparison to Current State

Today, to get a one-off crawl you have to:
1. Wait for the next daemon cycle, or
2. `myfeed once` which crawls all sites (slow) and sends everything to Telegram (no output to stdout), or
3. Run `pwright script run recipes/hackernews-feed.yaml` directly (pwright CLI, different output format)

With `myfeed crawl`:
```
myfeed crawl hackernews --compact
```
Gives you exactly the JSON you want, with the same recipe format the daemon uses.

## Next Steps

1. Implement `myfeed crawl` as new `Command::Crawl` variant in `main.rs`
2. Reuse `crawler.rs::run_recipe()` -- already returns `Vec<proto::FeedItem>`
3. Add `--format` flag for json/jsonl/table output
4. Add `--save` to persist to DB (reuse `db.rs::insert_item`)
5. Add `--notify` using existing `notifier::create_notifier()`
6. Add `myfeed recipe validate` as a dry-run command
7. Add `myfeed recipe list` that scans the recipes directory

## Open Questions

- Should `myfeed crawl --save` update `crawl_snapshots` table? Yes, for consistency with daemon behavior.
- Should `myfeed crawl` support sites with login (like `myfeed login` does)? Yes -- reuse the same Chrome session approach.
- Should `myfeed crawl` support `--param-file` for secrets (like pwright's `--param-file`)? Not needed -- auth is handled by Chrome session cookies, same as daemon.
