# Recipe Comparison: myfeed vs AutoCLI/opencli

<!-- agent-updated: 2026-04-05T05:30:00Z -->

## Philosophy

**myfeed/pwright: browser-first.** Everything goes through the browser. No API keys, no official APIs, no GraphQL intercepts. If a site works in Chrome, we can extract data from it. This is a deliberate design choice -- not a gap.

We do not add API-based fetching (Firebase, Algolia, Reddit JSON, Twitter GraphQL intercept) even when available. Browser-first means:
- One less thing to maintain when APIs change
- Works on any site regardless of API availability
- Consistent with how a human reads the site

**AutoCLI/opencli: API-first where available.** They use official APIs when accessible and fall back to DOM scraping. Faster and more reliable when APIs exist.

Neither approach is wrong -- they optimize for different things.

## Per-Site Feature Comparison

### Hacker News

| | myfeed | AutoCLI |
|--|--------|---------|
| Modes | 1 (front page) | 8 (top, new, best, ask, show, jobs, user, search) |
| Data source | DOM scrape | Firebase API |
| Comment previews | ✅ (batch-fetches top-30 discussion pages) | ❌ |
| User profiles | ❌ | ✅ |
| Search | ❌ | ✅ (Algolia API) |

**Note**: myfeed intentionally avoids HN's Firebase API. The browser approach is slower but produces richer output (top-3 comment text per story).

### Reddit

| | myfeed | AutoCLI |
|--|--------|---------|
| Modes | 1 (front page) | 15 (hot, frontpage, popular, subreddit, search, user, user-posts, user-comments, saved, upvoted, upvote, save, subscribe, comment) |
| Data source | DOM scrape | Reddit JSON API |
| Write actions | ❌ | ✅ (upvote, save, subscribe) |
| Subreddit filtering | ❌ | ✅ |
| User history | ❌ | ✅ |

**Note**: AutoCLI uses Reddit's public `/r/{sub}/hot.json` API. myfeed uses DOM scraping on the web page.

### Twitter/X

| | myfeed | AutoCLI |
|--|--------|---------|
| Modes | 0 (not supported) | 20+ (read + write: timeline, search, post, reply, like, bookmark, follow, etc.) |
| Data source | -- | GraphQL intercept |

**Note**: AutoCLI's intercept strategy captures Twitter's internal GraphQL API traffic. This is more reliable than DOM scraping. myfeed intentionally does not support Twitter due to the complexity of maintaining cookie-based auth + GraphQL query IDs.

### Xueqiu

| | myfeed | AutoCLI |
|--|--------|---------|
| Modes | 2 (home timeline, hot topics) | 7 (feed, hot, search, stock, watchlist, earnings-date, hot-stock) |
| Data source | DOM scrape | Xueqiu JSON APIs |
| Stock quotes | ✅ (ticker symbols only) | ✅ (full quote data) |
| Watchlist | ❌ | ✅ |

### Zhihu

| | myfeed | AutoCLI |
|--|--------|---------|
| Modes | 1 (hot listing) | 4 (hot, search, question, download) |
| Data source | DOM scrape | Zhihu API v3/v4 |
| Question/answer pages | ❌ | ✅ |
| Markdown export | ❌ | ✅ |
| Search | ❌ | ✅ |

### LinkedIn

| | myfeed | AutoCLI |
|--|--------|---------|
| Modes | 1 (personal feed, scroll-based) | 1 (job search via Voyager API) |
| Feed reading | ✅ | ❌ |
| Job search | ❌ | ✅ |
| Auth required | No | Yes |

**Note**: Split use case. myfeed reads the personal feed. AutoCLI does job search with pagination.

### Substack

| | myfeed | AutoCLI |
|--|--------|---------|
| Modes | 1 (discover/trending) | 3 (feed, search, publication) |
| Data source | DOM scrape | API + DOM |
| Category filter | ❌ | ✅ |
| Publication archive | ❌ | ✅ |

### V2EX

| | myfeed | AutoCLI |
|--|--------|---------|
| Modes | 1 (hot listing) | 10 (latest, hot, daily, node, topic, user, member, replies, notifications, nodes, me) |
| Data source | DOM scrape | V2EX JSON API |
| Topic replies | Via batch preview | ✅ |
| User profiles | ❌ | ✅ |
| Notifications | ❌ | ✅ |

---

## Key Takeaways

1. **AutoCLI has more features on every shared site.** More modes, API-based where available, write actions.

2. **myfeed's advantage is narrow**: HN comment previews (unique) and LinkedIn feed reading (AutoCLI doesn't do it).

3. **Browser-first means we don't compete on API-first sites.** For HN, Reddit, Twitter, Xueqiu -- AutoCLI's API-based approach is objectively better for those specific data points.

4. **myfeed's value is sites AutoCLI doesn't cover.** Finviz, Seeking Alpha, 36Kr, Weibo Hot, Caixin, CLS, LatePost, People's Daily, East Money, Futunn, Tildes, Slashdot, Douban, Telegram channels, GitHub Trending -- none of these are in AutoCLI/opencli.

5. **AutoCLI is a broader product** (55+ sites, 380+ commands, desktop app control). myfeed is a focused feed aggregator. They are not direct competitors for the same use case.

---

## Where myfeed Should Focus

**Recipe quality over quantity.** For sites both support:

- Make myfeed's version richer where it has unique data (HN comment previews)
- Don't try to match AutoCLI's mode count -- that's API work
- Instead, add the sites AutoCLI doesn't have: more Chinese finance/news, more niche feeds

**Recipe improvements worth keeping:**

- HN: Keep comment preview (unique feature)
- LinkedIn: Keep feed reading (unique use case)
- Any site AutoCLI doesn't have: prioritize these

**Recipe improvements that don't make sense:**

- Adding Twitter/X support (too much API complexity for marginal gain)
- Adding Twitter-style intercept strategies (not a browser-first approach)
- Matching AutoCLI's mode count via DOM scraping (fragile, high maintenance)
