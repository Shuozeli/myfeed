# Codelabs

## Lab 1: Set Up myfeed From Scratch

### Prerequisites

- Rust toolchain (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Chrome/Chromium
- A Telegram account

### Step 1: Start Chrome with Remote Debugging

Chrome needs a dedicated profile so login sessions persist between runs.

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

Verify it's running:
```bash
curl http://localhost:9222/json/version
```

If Chrome runs on a different machine, add `--remote-debugging-address=0.0.0.0`
and use that machine's IP/hostname as the CDP endpoint.

### Step 2: Create a Telegram Bot

1. Open Telegram, message [@BotFather](https://t.me/BotFather)
2. Send `/newbot`, pick a name and username
3. Copy the bot token

Get your chat ID:

1. Send any message to your bot
2. Open `https://api.telegram.org/bot<TOKEN>/getUpdates` in a browser
3. Find `"chat":{"id":YOUR_CHAT_ID}` in the JSON

For group chats: add the bot to the group, send a message, check
`getUpdates`. Group IDs are negative (e.g., `-1001234567890`).

### Important: Account and Anti-Bot Warnings

**Use separate accounts.** Create dedicated accounts for myfeed on each
site rather than using your personal accounts. Automated crawling may
trigger anti-bot detection, which could result in CAPTCHAs, temporary
locks, or permanent bans. A separate account isolates this risk from
your primary account.

**Anti-bot detection.** Sites actively detect automated browsing. myfeed
uses a real Chrome browser (not headless), which helps, but repeated
automated access patterns (same pages, regular intervals) can still
trigger detection. To reduce risk:

- Use longer crawl intervals (60+ minutes instead of the default 30)
- Don't run crawls 24/7 -- consider pausing overnight
- If you get CAPTCHAs or blocks, increase the interval or pause for a day
- Some sites (X, LinkedIn) are more aggressive than others

**myfeed is a personal tool for personal use.** It is not designed for
large-scale scraping. Respect each site's terms of service.

### Step 3: Clone and Configure

```bash
git clone https://github.com/shuozeli/myfeed.git
cd myfeed
cp .env.example .env
```

Edit `.env`:
```bash
CDP_ENDPOINT=http://localhost:9222
DATABASE_URL=myfeed.db
TELEGRAM_BOT_TOKEN=<your token>
TELEGRAM_CHAT_ID=<your chat id>
CRAWL_INTERVAL_SECS=1800
ENABLED_SITES=hackernews,reddit
```

Start with one or two sites. Add more once you've verified things work.

### Step 4: Build and Log In

```bash
cargo build --release

# Log in to sites that require authentication
./target/release/myfeed login reddit
# Chrome opens the login page -- log in manually, press Enter

./target/release/myfeed login zhihu
# Repeat for each site
```

Hacker News works without login. Sites like Reddit, Zhihu, X, LinkedIn
require you to log in once -- session cookies persist in Chrome's profile.

### Step 5: Test a Single Crawl

```bash
./target/release/myfeed once
```

Check your Telegram -- you should see messages arriving. Check events:

```bash
./target/release/myfeed events -l 10
```

If a recipe fails (e.g., selectors changed), you'll see `crawl_error`
events with details.

### Step 6: Run the Daemon

```bash
./target/release/myfeed run
```

This crawls all enabled sites every 30 minutes. Run it in tmux, screen,
or as a systemd service:

```ini
# /etc/systemd/system/myfeed.service
[Unit]
Description=myfeed
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

```bash
sudo systemctl enable --now myfeed
```

---

## Lab 2: Add a New Site

### Step 1: Explore the Site

Create an exploration recipe to understand the DOM:

```yaml
# recipes/explore/mysite-explore.yaml
name: "MySite Explore"
version: 1
type: query
config:
  default_timeout_ms: 20000
scripts:
  dump_structure: |
    (() => {
      const result = { url: location.href, selectors: [], samples: [] };
      const checks = ['article', '[class*="post"]', '[class*="card"]',
                       'a[href*="/post/"]', 'table tr'];
      for (const sel of checks) {
        const c = document.querySelectorAll(sel).length;
        if (c > 0) result.selectors.push({ sel, c });
      }
      const els = document.querySelectorAll('article, [class*="post"]');
      for (let i = 0; i < Math.min(els.length, 3); i++) {
        result.samples.push({
          tag: els[i].tagName,
          classes: els[i].className,
          text: els[i].textContent.trim().substring(0, 200)
        });
      }
      return JSON.stringify(result, null, 2);
    })()
steps:
  - goto: "https://mysite.com"
    wait_for: "body"
  - wait: 3000
  - eval: { ref: dump_structure, save_as: structure }
  - output: { structure: "{{ structure }}" }
```

Run it:
```bash
pwright script run recipes/explore/mysite-explore.yaml
```

Review the output to identify selectors and content patterns.

### Step 2: Write the Feed Recipe

```yaml
# recipes/mysite-feed.yaml
name: "MySite Feed"
version: 1
type: query
config:
  default_timeout_ms: 30000
  default_on_error: continue
scripts:
  extract: |
    (() => {
      const posts = document.querySelectorAll('THE_SELECTOR_YOU_FOUND');
      const results = [];
      for (const post of posts) {
        const title = post.querySelector('h2 a')?.textContent?.trim() || '';
        const url = post.querySelector('h2 a')?.href || '';
        const id = url.match(/\/post\/(\w+)/)?.[1] || title;
        const preview = post.querySelector('p')?.textContent?.trim() || '';
        if (id && title) results.push({ id, title, url, preview });
      }
      return JSON.stringify(results);
    })()
steps:
  - goto: "https://mysite.com"
    wait_for: "THE_SELECTOR_YOU_FOUND"
  - wait: 2000
  - eval: { ref: extract, save_as: items }
  - output: { items: "{{ items }}" }
```

Test it:
```bash
pwright script run recipes/mysite-feed.yaml
```

### Step 3: Add the Proto Variant (optional)

If you want typed per-site data, add a message to `proto/myfeed.proto`:

```protobuf
message MySiteData {
  string author = 1;
  int32 upvotes = 2;
}
```

Add it to the `FeedItem` oneof and update `crawler.rs` to map the fields.

### Step 4: Enable the Site

Add to `.env`:
```
ENABLED_SITES=hackernews,reddit,mysite
```

Add the login URL to `src/main.rs` if the site requires authentication.

---

## Lab 3: Configure Crawl Schedule and Notifications

### Change Crawl Frequency

Edit `.env`:
```bash
CRAWL_INTERVAL_SECS=900   # Every 15 minutes
CRAWL_INTERVAL_SECS=3600  # Every hour
CRAWL_INTERVAL_SECS=1800  # Every 30 minutes (default)
```

Restart the daemon after changing.

### Enable/Disable Sites

```bash
# Crawl only HN and Reddit
ENABLED_SITES=hackernews,reddit

# Add more sites
ENABLED_SITES=hackernews,reddit,zhihu,x,linkedin,xueqiu,1point3acres
```

### Filter Notifications by Keyword

Only get notified about items you care about. All items are still saved
to the database for snapshots and agent digests.

```bash
# Only notify on AI, Rust, immigration, and interview topics
FILTER_KEYWORDS=AI,rust,immigration,interview,tariff

# Case-insensitive, matches against title and preview text
# Unset or empty = no filter (notify on everything)
```

### Use Digest Mode

Instead of one Telegram message per item, get a single summary message
per site per crawl cycle:

```bash
DIGEST_MODE=true
```

Digest format:
```
[hackernews] 5 new items

- Flash-MoE: Running a 397B Parameter Model on a Laptop
- The Future of Version Control
- ...
```

Set `DIGEST_MODE=false` (default) for individual messages.

### Generate Atom Feed

Serve your feed via any RSS reader:

```bash
FEED_OUTPUT_PATH=feed.xml
FEED_ITEM_COUNT=100
```

After each crawl cycle, `feed.xml` is regenerated. Serve it with nginx,
caddy, or `python -m http.server`.

### Customize Telegram Message Format

Edit `src/telegram.rs` `send_feed_item()` to change the per-item format,
or edit `scheduler.rs` `format_digest()` to change the digest format.

### Query History

```bash
# List recent items
myfeed list -l 20

# Filter by site
myfeed list --site reddit -l 10

# Check crawl snapshots
myfeed snapshots hackernews

# View a specific snapshot
myfeed snapshot 42

# Debug events
myfeed events -l 20
```

---

## Lab 4: Debug a Failing Recipe

### Check the Event Log

```bash
myfeed events -l 10
```

Look for `crawl_error` events. Common issues:

| Symptom | Cause | Fix |
|---------|-------|-----|
| `recipe not found` | Site name doesn't match filename | `reddit` -> `recipes/reddit-feed.yaml` |
| `Element not found` | Site changed HTML structure | Run explore recipe, update selectors |
| `timeout` | Page didn't load | Increase `timeout_ms` or add longer `wait` |
| `failed to connect` | Chrome not running | Start Chrome with `--remote-debugging-port` |
| 0 items found | Login expired | Run `myfeed login <site>` |

### Test a Recipe in Isolation

```bash
# Run just the recipe, see raw JSONL output
pwright script run recipes/reddit-feed.yaml

# Validate recipe syntax without executing
pwright script validate recipes/reddit-feed.yaml
```

### Inspect the DOM Live

```bash
# Open a page and explore
pwright open https://example.com

# Evaluate JS on the current page
pwright eval 'document.querySelectorAll("article").length'

# Take a snapshot
pwright snapshot
```

---

## Lab 5: Follow Accounts and Join Channels

myfeed includes action recipes for subscribing to content sources:

```bash
# Follow a company on LinkedIn
pwright script run recipes/actions/follow-linkedin.yaml \
  --param url=https://www.linkedin.com/company/google/

# Follow someone on X
pwright script run recipes/actions/follow-x.yaml \
  --param url=https://x.com/AnthropicAI

# Join a subreddit
pwright script run recipes/actions/join-reddit.yaml \
  --param url=https://www.reddit.com/r/rust/
```

These use the same Chrome session with your logged-in cookies.

---

## Lab 6: AI Agent Digest

Use an AI agent (Claude Code, Gemini CLI, etc.) to summarize your feeds.
No LLM calls are baked into myfeed -- the agent reads the data and
applies intelligence externally.

### Step 1: Scan the index

```bash
myfeed dump --hours 24 --compact
```

This outputs a compact JSON index (~10 tokens per item) with `id`,
`site`, `title`, `url`. The agent reads titles to decide what's
interesting.

### Step 2: Read details for selected items

```bash
myfeed dump --ids 42,55,78,91
```

Returns full `preview`, `raw_json`, and `created_at` for those items.

### Step 3: Apply a prompt template

Prompt templates are in `prompts/`:

- `daily-digest.md` -- Top stories + topic clusters
- `trending-topics.md` -- What's trending across sites
- `tech-radar.md` -- Tech-focused summary

The agent reads the template and the dump data, then produces the digest.

### Example with Claude Code

```
You: Summarize my feeds from today

Claude: [runs myfeed dump --hours 24 --compact]
        [scans 200 titles, picks 15 interesting items]
        [runs myfeed dump --ids 12,45,67,...]
        [reads full content, produces summary]

        Top Stories:
        - Danone acquiring Huel for $1.2B (X, HN)
        - 2026 tariffs costing US households $570/yr (1point3acres)
        ...
```

See `docs/agent-digest-guide.md` for the full guide.
