# myfeed

English | [中文](README.zh-CN.md)

An alternative to RSS for sites that don't support it. myfeed connects to a Chrome browser, extracts posts using YAML recipes, and sends new items to Telegram.

It works by automating a real browser tab via CDP -- using your logged-in Chrome sessions, so it can read login-gated pages. Recipes are YAML files with JS extraction logic. Adding a site means writing one file, no Rust changes.

```
Chrome (your sessions)  -->  YAML recipes  -->  SQLite dedup  -->  Telegram
```

## How it works

- Connects to Chrome via CDP (uses your existing sessions and cookies)
- Runs YAML recipes on a schedule (default: every 30 min)
- Deduplicates items in SQLite, sends new ones to Telegram
- Recipes can be written and maintained by AI agents -- most take minutes to create
- No LLM tokens at runtime; recipes are deterministic once written

## Supported sites (23 public recipes)

| Category | Sites |
|----------|-------|
| Tech | Hacker News, Reddit, V2EX, Slashdot, Tildes, InfoQ, GitHub Trending, Substack |
| Social | X (Twitter), LinkedIn, Telegram Channels, Douban |
| Finance | Xueqiu, East Money, Futunn, Finviz, Seeking Alpha |
| Chinese | Zhihu, 1point3acres, Weibo, 36Kr |

Private recipes (gitignored) can be added for additional sites.

## Quick start

```bash
# 1. Start Chrome with remote debugging
google-chrome --remote-debugging-port=9222 --user-data-dir=$HOME/.myfeed-chrome

# 2. Clone and build
git clone https://github.com/Shuozeli/myfeed.git && cd myfeed
cp .env.example .env   # edit with your Telegram bot token + chat ID
cargo build --release

# 3. Log in to sites (one-time)
./target/release/myfeed login reddit
./target/release/myfeed login zhihu

# 4. Run
./target/release/myfeed run   # crawls every 30 min, sends new posts to Telegram
```

## How a recipe looks

```yaml
# recipes/hackernews-feed.yaml
steps:
  - goto: "https://news.ycombinator.com"
    wait_for: ".athing"
  - eval:
      ref: extract_stories    # JS function that returns [{id, title, url, preview}]
      save_as: items
  - output:
      items: "{{ items }}"
```

Each recipe navigates to a page, waits for content to load, runs JS to extract items, and outputs a JSON array. The contract is simple: `{id, title, url, preview}`.

## Adding a new site

1. Create `recipes/<site>-feed.yaml` with JS that extracts `[{id, title, url, preview}]`
2. Add the site name to `ENABLED_SITES` in `.env`
3. Done. No Rust changes needed.

For login-gated sites, run `myfeed login <site>` once. Session cookies persist in Chrome's profile.

Have a site to request? [Open an issue](https://github.com/Shuozeli/myfeed/issues/new?template=new-site-recipe.yml).

## Architecture

```
src/
  main.rs        CLI: run, once, login, list, events, dump
  config.rs      All settings from env vars (fail-fast on missing)
  crawler.rs     Runs pwright recipes, parses output into typed FeedItems
  scheduler.rs   Async loop: crawl -> snapshot -> dedup -> telegram
  db.rs          SQLite via diesel. All queries in transactions.
  telegram.rs    Message queue with rate limiting (1 msg/sec, 429 backoff)
  feed.rs        Generates Atom 1.0 XML from feed_items

recipes/         One YAML file per site. JS extraction logic, no Rust.
proto/           Protobuf schema with per-site typed payloads
```

## Agent integration

The `dump` command exposes feed data for AI agents:

```bash
myfeed dump --hours 24 --compact          # scan titles (~10 tokens/item)
myfeed dump --ids 42,55,78                # full details for selected items
```

Prompt templates in `prompts/` for daily digests, trending topics, and tech radar. See [agent digest guide](docs/agent-digest-guide.md).

## Configuration

| Variable | Description |
|----------|-------------|
| `CDP_ENDPOINT` | Chrome DevTools HTTP URL (e.g., `http://localhost:9222`) |
| `DATABASE_URL` | SQLite path (e.g., `myfeed.db`) |
| `TELEGRAM_BOT_TOKEN` | From [@BotFather](https://t.me/BotFather) |
| `TELEGRAM_CHAT_ID` | Target chat ID |
| `CRAWL_INTERVAL_SECS` | Seconds between cycles (suggested: `1800`) |
| `ENABLED_SITES` | Comma-separated site names |
| `FILTER_KEYWORDS` | Optional: only notify on matching items |
| `DIGEST_MODE` | Optional: batch into one message per site |
| `DEDUP_WINDOW_HOURS` | Optional: re-notify after N hours (0 = never) |
| `FEED_OUTPUT_PATH` | Optional: write Atom feed XML |

All required variables panic on missing -- no silent defaults.

## Dependencies

Built on [pwright](https://github.com/shuozeli/pwright) (Chrome CDP bridge + recipe engine), diesel (SQLite), tokio, reqwest.

## Demo

We run myfeed on a home server crawling every 30 minutes. A full cycle takes ~5 minutes and typically finds 50-150 new items. Here's a real 12-hour snapshot: [demo-snapshot.md](docs/demo-snapshot.md) -- 1,400+ items across 20 sites, all from the public recipes in this repo.

## License

MIT
