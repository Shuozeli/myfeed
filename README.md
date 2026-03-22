# myfeed

A personal feed bot that crawls your social media timelines and delivers new posts to Telegram. Built on [pwright](https://github.com/shuozeli/pwright), a Rust browser automation library that controls Chrome via CDP.

## Why myfeed?

**The problem:** You follow content across Reddit, Zhihu, Weibo, and X, but checking four apps throughout the day is noisy and distracting. APIs are unreliable -- platforms deprecate them, gate them behind paid tiers, or restrict what you can access. And your personal logged-in feed (the content you actually curated) is never available through APIs at all.

**What myfeed does:**

- **Sees exactly what you see.** It drives a real Chrome browser with your logged-in sessions. No API keys, no OAuth tokens, no rate limits. If you can see it in the browser, myfeed can extract it.
- **One notification channel.** New posts from all sites arrive in a single Telegram chat. Check once, not four times.
- **No vendor lock-in.** Site-specific logic lives in YAML recipe files, not compiled code. When a site redesigns, update the recipe -- no Rust recompilation needed.
- **Runs anywhere Chrome runs.** A laptop, a Raspberry Pi, a VPS. Just Chrome + the myfeed binary.
- **Deduplicates automatically.** SQLite tracks what you've already seen. Restart the daemon, re-run a crawl -- you'll never get duplicate notifications.

**Who is this for?** Anyone who wants a personal, self-hosted feed aggregator that works with login-gated content and doesn't depend on third-party APIs.

## Supported Sites

| Site | What it crawls | Recipe |
|------|---------------|--------|
| Reddit | Front page / subscribed subreddits | `recipes/reddit-feed.yaml` |
| Hacker News | Top stories with comments | `recipes/hackernews-feed.yaml` |
| Zhihu | Hot questions and articles | `recipes/zhihu-feed.yaml` |
| X (Twitter) | Home timeline | `recipes/x-feed.yaml` |
| LinkedIn | Feed posts | `recipes/linkedin-feed.yaml` |
| Xueqiu | Market discussions | `recipes/xueqiu-feed.yaml` |
| 1point3acres | Forum threads with post content | `recipes/1point3acres-feed.yaml` |

Adding a new site takes one YAML file. See [Adding a New Site](#adding-a-new-site).

## How It Works

```
  You log in once           myfeed crawls on a schedule          New posts go to Telegram
  via Chrome tab     -->    using your browser session     -->   as formatted messages
       |                           |                                    |
  [Chrome CDP]           [pwright recipes]                     [Telegram Bot API]
                                   |
                          [SQLite dedup]
```

1. Start Chrome with remote debugging enabled
2. Run `myfeed login reddit` -- a tab opens, you log in manually, press Enter
3. Run `myfeed run` -- the daemon crawls every 30 minutes (configurable)
4. New posts arrive in your Telegram chat

Session cookies persist in Chrome's user data directory. You only log in once per site.

## Setup Guide

### Step 1: Install Prerequisites

You need three things: Rust, Chrome, and a Telegram bot.

**Rust:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Chrome with remote debugging:**
```bash
# Linux
google-chrome \
  --remote-debugging-port=9222 \
  --user-data-dir=$HOME/.myfeed-chrome

# macOS
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome \
  --remote-debugging-port=9222 \
  --user-data-dir=$HOME/.myfeed-chrome

# Windows
chrome.exe --remote-debugging-port=9222 --user-data-dir=%USERPROFILE%\.myfeed-chrome
```

`--user-data-dir` creates a dedicated Chrome profile for myfeed so login sessions persist between runs and don't interfere with your daily browsing.

**Use separate accounts.** Create dedicated accounts on each site for
myfeed rather than using your personal accounts. Automated browsing may
trigger anti-bot detection (CAPTCHAs, temporary locks, or bans). A
separate account keeps your primary account safe. See [codelabs](docs/codelabs.md)
for details.

**Verify Chrome is accepting connections:**
```bash
curl -s http://localhost:9222/json/version
# Should return JSON with "webSocketDebuggerUrl"
```

### Step 2: Create a Telegram Bot

1. Open Telegram and message [@BotFather](https://t.me/BotFather)
2. Send `/newbot`
3. Follow the prompts -- pick a name and username
4. BotFather replies with a **token** like `YOUR_BOT_TOKEN`
5. Save this token -- you'll need it in Step 3

**Get your chat ID:**

1. Send any message to your new bot (e.g., "hello")
2. Open this URL in a browser (replace `<TOKEN>` with your bot token):
   ```
   https://api.telegram.org/bot<TOKEN>/getUpdates
   ```
3. In the JSON response, find `"chat":{"id":YOUR_CHAT_ID}`
4. Save this number -- it's your chat ID

**For group chats:** Add the bot to the group, send a message mentioning the bot, then check `getUpdates`. The group chat ID will be negative (e.g., `-1001234567890`).

### Step 3: Configure myfeed

```bash
git clone https://github.com/shuozeli/myfeed.git
cd myfeed
cp .env.example .env
```

Edit `.env`:

```bash
# Chrome CDP -- must match the port Chrome is listening on
CDP_ENDPOINT=ws://localhost:9222

# SQLite database file (created automatically)
DATABASE_URL=myfeed.db

# From Step 2
TELEGRAM_BOT_TOKEN=YOUR_BOT_TOKEN
TELEGRAM_CHAT_ID=987654321

# Crawl every 30 minutes (in seconds)
CRAWL_INTERVAL_SECS=1800

# Which sites to crawl (comma-separated)
ENABLED_SITES=reddit,hackernews,zhihu,x,linkedin,xueqiu,1point3acres
```

Every variable is required. If any is missing, myfeed fails immediately at startup with a clear error message.

### Step 4: Build

```bash
cargo build --release
```

### Step 5: Log In to Sites

For each site you want to crawl, run the login command. It opens a Chrome tab -- you log in manually, then press Enter.

```bash
./target/release/myfeed login reddit
# Browser opens to reddit.com/login
# Log in, then press Enter in the terminal

./target/release/myfeed login zhihu
./target/release/myfeed login x
./target/release/myfeed login linkedin
./target/release/myfeed login xueqiu
./target/release/myfeed login 1point3acres
# Hacker News works without login
```

Your session cookies are saved in Chrome's profile directory. You only need to do this once (or when a session expires).

### Step 6: Test

```bash
# Run a single crawl cycle
./target/release/myfeed once

# Check if items were found
./target/release/myfeed events -l 10
```

You should see `crawl_start` and `crawl_complete` events, and new posts arriving in Telegram.

### Step 7: Run the Daemon

```bash
./target/release/myfeed run
```

myfeed will crawl all enabled sites every 30 minutes and send new posts to Telegram. Run it in a tmux/screen session, as a systemd service, or however you prefer to run long-lived processes.

**Example systemd service** (`/etc/systemd/system/myfeed.service`):

```ini
[Unit]
Description=myfeed personal feed bot
After=network.target

[Service]
Type=simple
User=youruser
WorkingDirectory=/home/youruser/myfeed
ExecStart=/home/youruser/myfeed/target/release/myfeed run
EnvironmentFile=/home/youruser/myfeed/.env
Restart=on-failure
RestartSec=30

[Install]
WantedBy=multi-user.target
```

## CLI Reference

| Command | Description |
|---------|-------------|
| `myfeed run` | Start the daemon -- crawl on a schedule, send to Telegram |
| `myfeed once` | Run one crawl cycle and exit (for testing) |
| `myfeed login <site>` | Open Chrome tab for manual login (`reddit`, `zhihu`, `x`, `linkedin`, `xueqiu`, `hackernews`, `1point3acres`) |
| `myfeed events -l N` | Show the last N events from the debug log (default: 20) |

## Configuration Reference

| Variable | Description | Example |
|----------|-------------|---------|
| `CDP_ENDPOINT` | Chrome DevTools WebSocket URL | `ws://localhost:9222` |
| `DATABASE_URL` | SQLite database path | `myfeed.db` |
| `TELEGRAM_BOT_TOKEN` | Bot token from BotFather | `YOUR_BOT_TOKEN` |
| `TELEGRAM_CHAT_ID` | Target chat or group ID | `987654321` |
| `CRAWL_INTERVAL_SECS` | Seconds between crawl cycles | `1800` |
| `ENABLED_SITES` | Comma-separated site names | `reddit,hackernews,zhihu,x,linkedin,xueqiu,1point3acres` |

## Adding a New Site

Adding a site requires zero Rust code changes -- just YAML recipes.

**1. Explore the site's HTML structure:**

Create `recipes/explore/<site>-explore.yaml` with JS that dumps the DOM structure, then run it:
```bash
pwright script run recipes/explore/<site>-explore.yaml
```

**2. Write the feed recipe:**

Create `recipes/<site>-feed.yaml`. The recipe must output items as a JSON array with this contract:

```json
[
  {
    "id": "unique-dedup-key",
    "title": "Post title or first 80 chars",
    "url": "https://link-to-original",
    "preview": "First 200 chars of content"
  }
]
```

See existing recipes in `recipes/` for examples.

**3. Enable the site:**

Add the site name to `ENABLED_SITES` in `.env`. If login is required, add the login URL to the `Login` command match in `src/main.rs`.

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| "missing required env var" at startup | `.env` incomplete | Check all 6 variables are set |
| "failed to connect to Chrome" | Chrome not running or wrong port | Start Chrome with `--remote-debugging-port=9222` |
| Crawl completes but 0 items found | Site changed HTML structure | Run the explore recipe, update selectors in feed recipe |
| Telegram messages not arriving | Wrong bot token or chat ID | Re-check via `getUpdates` API |
| "recipe not found, skipping" | Site name in `ENABLED_SITES` doesn't match recipe filename | `reddit` -> `recipes/reddit-feed.yaml` |
| Login expired | Session cookies cleared | Run `myfeed login <site>` again |

## Project Structure

```
myfeed/
  src/
    main.rs          # CLI (run, once, login, events)
    config.rs        # Env-based config, fail-fast on missing
    db.rs            # SQLite via diesel -- dedup + event log
    schema.rs        # Diesel schema (auto-generated)
    crawler.rs       # Runs pwright recipes, parses output
    scheduler.rs     # Periodic crawl loop
    telegram.rs      # Telegram Bot API client
  recipes/
    explore/         # HTML structure discovery (run before writing feed recipes)
    reddit-feed.yaml # Feed extraction recipes (one per site)
    zhihu-feed.yaml
    weibo-feed.yaml
    x-feed.yaml
  migrations/        # Diesel SQL migrations
  docs/              # Architecture, design, codelabs
```

## License

MIT
