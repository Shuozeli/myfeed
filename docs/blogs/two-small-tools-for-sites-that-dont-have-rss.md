# Two Small Tools for Sites That Don't Have RSS

## The problem everyone has and nobody talks about

You want to follow a site. It doesn't have RSS. Your options are:

- **Check it manually.** Bookmark it, remember to open it, waste time on it when you don't need to.
- **Use a scraping service.** Someone else's infrastructure, someone else's browser sessions, someone else's terms of service.
- **Build a scraper.** Write it once, maintain it every time the site changes, throw it away when it breaks.

This is not a new problem. RSS was invented because sites shouldn't have to build email newsletters for content that should be syndication. But RSS has limits -- it only works if sites support it, and most don't. Not won't, don't. The economics of running a public API or RSS feed don't make sense for every site, especially smaller ones.

The result is a fragmented experience: your RSS reader is clean and quiet, but mostly empty. The rest of what you actually want to read lives in tabs you never open.

## What existing solutions get wrong

A few categories of existing solutions:

**RSS aggregators with scraping built in** (Inoreader, Feedly, etc.)
handle the RSS part well but treat scraping as an afterthought. The output is often degraded -- you get a title and a link, not the actual content. And you're still relying on a hosted service that you don't control.

**Open-source RSS bridges** (rss-bridge, FiveFilters) work for some sites and are self-hostable, but they're fragile. A site changes its HTML, the bridge breaks, someone files a bug, maybe it gets fixed. The maintenance burden falls on volunteers who may not care about your specific site.

**Third-party monitoring services** (ChangeDetector, Distill.io) are designed for tracking price changes and product availability, not following editorial content. The UX reflects this -- dashboards, alerts, change notifications. Not a feed.

**Browser-based AI agents** are the newest category. Tools like Browserbase or direct MCP integrations let an agent operate a browser on your behalf. The agent can figure out any site -- no recipe needed. This is genuinely powerful, but it comes with tradeoffs: every session involves multiple LLM round-trips, token costs add up, and results vary depending on how well the agent interprets the page at that moment. For repetitive, predictable tasks (reading the same HN front page every day), it's expensive overkill.

The common thread: most solutions optimize for either reach (any site) or reliability (same result every time). Not both.

## The approach: recipes as a middle ground

The idea behind both tools is straightforward: encode "how to read this page" as a data file, not a script that runs in an LLM's context.

A recipe is a YAML file. It says: navigate to this URL, wait for this selector to appear, run this JavaScript to extract the data, return it as JSON. That's it.

Here's a recipe for Hacker News:

```yaml
# hackernews-feed.yaml
name: "Hacker News Feed"
type: query
scripts:
  extract_stories: |
    (() => {
      const rows = document.querySelectorAll('.athing');
      return JSON.stringify([...rows].map(row => {
        const titleEl = row.querySelector('.titleline > a');
        const subtext = row.nextElementSibling;
        return {
          title: titleEl?.textContent?.trim(),
          url: titleEl?.href,
          points: parseInt(subtext?.querySelector('.score')?.textContent) || 0,
          comments: parseInt(subtext?.querySelectorAll('a').pop()?.textContent) || 0
        };
      }));
    })()
steps:
  - goto: "https://news.ycombinator.com"
    wait_for: ".athing"
  - eval: { ref: extract_stories, save_as: stories }
  - output: { stories: "{{ stories }}" }
```

Run it:

```
pwright script run hackernews-feed.yaml
```

In about 2 seconds you get back:

```json
{
  "stories": [
    {"title": "Show HN: We put a whole company in a single HTML file", "url": "https://example.com", "points": 312, "comments": 89},
    {"title": "Rust in production: one year later", "url": "https://example.com", "points": 187, "comments": 43},
    ...
  ]
}
```

No screenshots. No accessibility tree. No LLM reasoning about where the upvote button is. The recipe already knows.

### Why recipes over raw browser control

The comparison to general-purpose browser agents is worth making explicit.

A general agent navigating a page: takes a snapshot, reasons about the structure, decides what to click, takes another snapshot, extracts the data. This works. It's also slow and expensive for tasks that are the same every time.

The Hacker News front page has not changed its DOM structure in a meaningful way in years. `.athing` for story rows, `.titleline > a` for links, `.score` for points. These are stable facts. Encoding them in a recipe means the agent never has to figure them out again.

Recipes don't replace agents. They make agents faster on the cases they already know how to handle.

This isn't a philosophical point about AI. It's a practical one: for the 80% of browser tasks that are repetitive and predictable, deterministic execution is better than on-demand reasoning. For the 20% that are novel, agents can still fall back to runtime exploration.

### Writing recipes is not hard

The JS in a recipe is just DOM traversal. If you've ever opened Chrome DevTools and typed `document.querySelector` in the console, you can write a recipe.

For most sites, the workflow is:

1. Open the site in Chrome, inspect the DOM
2. Find the selectors for the content you want (article titles, timestamps, URLs)
3. Write a JS function that extracts them
4. Test it in the browser console first
5. Package it as a YAML recipe

This takes 10-20 minutes for a well-structured site. We've found that AI agents can do it reliably given a few examples. The agent doesn't need to understand the page -- it just needs to follow the pattern from an existing recipe.

## myfeed: recipes on a schedule

[pwright](https://github.com/shuozeli/pwright) is the recipe engine. [myfeed](https://github.com/shuozeli/myfeed) is the thing that uses it for personal feed aggregation.

myfeed runs recipes on a schedule, deduplicates new items, and sends them to Telegram. It's a personal RSS reader built on top of pwright.

The setup:

```bash
# Start Chrome with remote debugging
google-chrome --remote-debugging-port=9222 --user-data-dir=$HOME/.myfeed-chrome

# Configure: which sites, how often, where to send
ENABLED_SITES=hackernews,reddit,v2ex,github-trending
CRAWL_INTERVAL_SECS=1800

# Run
./myfeed run
```

Adding a new site means writing one YAML recipe and adding its name to `ENABLED_SITES`. No code changes.

### What you actually get

On a real instance running on a home server (crawling every 30 minutes, 20 sites):

- A full cycle takes 4-6 minutes end-to-end
- Most sites produce 0-2 new items per cycle
- High-turnover feeds (trending topics, stock tickers) produce more
- All items are stored in SQLite for deduplication across cycles
- Telegram receives only genuinely new items

The [demo snapshot](https://github.com/shuozeli/myfeed/blob/main/docs/demo-snapshot.md) shows real output from a 12-hour run so you can see what the data actually looks like before running anything.

## What this is not

**Not a hosted service.** You run it on your own machine or a cheap VPS. Chrome has to be running. There is no "set it and forget it" cloud offering.

**Not API-based.** We don't use site APIs. Everything goes through the browser. This is a limitation -- APIs are more reliable when they exist -- but it's also the point. Browser automation works on any site that works in Chrome, including sites that have no developer support at all.

**Not maintenance-free.** Recipes break when sites redesign. The error is clear (the selector doesn't exist), but someone has to fix it. We use this ourselves daily, so regressions get noticed fast, but this is real work.

**Not for every site.** Recipes work for sites with reasonably stable markup. Internal tools, documentation, public pages with simple DOM structures -- these are good fits. Sites with heavy anti-bot protection or frequent redesigns are not.

## The honest pitch

We built this because we wanted it. The tools solve a real problem for us, and we're sharing them in case they solve it for someone else.

The repos:

- [pwright](https://github.com/shuozeli/pwright) -- MIT licensed, recipe engine + CLI
- [myfeed](https://github.com/shuozeli/myfeed) -- MIT licensed, feed aggregator

Both are small, self-contained, and do one thing. If they happen to do the thing you need, great. If not, the recipe format is simple enough that you can adapt it.
