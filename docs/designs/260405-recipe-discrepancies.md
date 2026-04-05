# Recipe Discrepancies Setup

<!-- agent-updated: 2026-04-05T18:00:00Z -->

## Overview

Systematic approach to tracking and improving recipes where myfeed differs from AutoCLI. Focus on maintaining unique advantages rather than competing on API-based features.

## Philosophy

**myfeed's edge is unique data, not more modes.**

- HN: Keep batch comment previews (AutoCLI doesn't do this)
- LinkedIn: Keep scroll-based feed reading (AutoCLI doesn't do this)
- Chinese finance/news: Maintain coverage AutoCLI lacks

**AutoCLI's API-first approach is objectively better for:**
- HN Firebase API (faster, more reliable than DOM)
- Reddit JSON API (structured, no scraping)
- Twitter GraphQL intercept (more reliable than DOM)

We intentionally do NOT compete on these. Browser-first is a design principle.

## Recipe Categories

### Category 1: Unique Data (myfeed wins)

These sites/features AutoCLI doesn't cover at all, or doesn't do well:

| Site | Unique Feature | Status |
|------|---------------|--------|
| HackerNews | Batch comment previews (top 3 per story) | ✅ Good |
| LinkedIn | Scroll-based personal feed reading | ✅ Good |
| 36Kr | Full article body extraction | ✅ Good |
| CLS (财联社) | Chinese financial news | ✅ Good |
| LatePost | Chinese investigative journalism | ✅ Good |
| People's Daily | Chinese official news | ✅ Good |
| East Money | Chinese financial data | ✅ Good |
| Futunn | Chinese fintech | ✅ Good |
| Caixin | Chinese business journalism | ✅ Good |
| Finviz | US stock screener | ✅ Good |
| Seeking Alpha | Financial analysis | ✅ Good |
| Tildes | Open-source aggregator | ✅ Good |
| Slashdot | Tech news aggregator | ✅ Good |
| Douban | Chinese social recommendations | ✅ Good |
| GitHub Trending | Dev activity tracking | ✅ Good |
| Telegram Channels | Public channel content | ✅ Good |

### Category 2: API Gap (AutoCLI wins)

These sites have APIs AutoCLI uses but myfeed intentionally doesn't:

| Site | AutoCLI Uses | myfeed Uses | Decision |
|------|-------------|-------------|----------|
| HackerNews | Firebase API | DOM + comment fetch | Keep DOM (unique data) |
| Reddit | Reddit JSON API | DOM scraping | Keep DOM (philosophy) |
| Xueqiu | Xueqiu JSON API | DOM scraping | Keep DOM (philosophy) |
| V2EX | V2EX JSON API | DOM scraping | Keep DOM (philosophy) |
| Twitter/X | GraphQL intercept | Not supported | Intentional (too complex) |

**Rationale:** Browser-first means accepting slower/less reliable for consistency and zero API maintenance.

### Category 3: Missing Coverage

Sites AutoCLI has that myfeed doesn't (and should consider):

| Site | AutoCLI Modes | Notes |
|------|--------------|-------|
| Twitter/X | 20+ (timeline, search, post, etc.) | Not worth the complexity |
| Instagram | Feed, search, post | Auth-heavy, low priority |
| YouTube | Trending, search, comments | Could add |

## Recipe Quality Checklist

For each recipe, ensure it produces **richer output than API-based alternatives**:

### Must Have
- [ ] Extracts id, title, url, preview
- [ ] preview contains actual content (not just truncated title)
- [ ] site_data contains relevant metrics (score, comments, upvotes, etc.)

### HN-Specific (Unique Advantage)
- [ ] Batch fetches top 30 discussion pages
- [ ] Extracts top 3 comments per story
- [ ] preview contains comment text, not just metadata

### LinkedIn-Specific (Unique Advantage)
- [ ] Scrolls to load more posts
- [ ] Extracts author, company, full post text
- [ ] Extracts likes, comments engagement numbers

### Chinese Finance Sites
- [ ] Fetches full article body for top items
- [ ] preview contains 200-500 chars of body text
- [ ] Works with Chinese character encoding

## Recipe Testing

Test each recipe after changes:

```bash
# Validate recipe syntax
myfeed recipe validate <site>

# Crawl and check output
myfeed crawl <site> --format json --limit 5

# Check for empty items or parsing errors
myfeed crawl <site> --format json | jq '.[] | select(.preview == null or .preview == "")'
```

## Maintenance

When a recipe breaks:

1. Check if site HTML structure changed
2. Run `pwright script run recipes/explore/<site>-explore.yaml` to understand new structure
3. Update recipe selectors
4. Test with `myfeed crawl <site> --save`
5. Verify items appear in `myfeed list --site <site>`

## File Structure

```
recipes/
  # Public recipes (32 sites)
  <site>-feed.yaml

  # Test recipes
  test/
    simple-feed.yaml
    hackernews-mock.yaml
    reddit-mock.yaml

  # Explore scripts (HTML structure discovery)
  explore/
    <site>-explore.yaml

  # Action recipes (follow, join, etc.)
  actions/
    follow-linkedin.yaml
    follow-x.yaml
    join-reddit.yaml
```

## Gap Analysis Summary

Based on AutoCLI comparison:

**Where myfeed wins:**
- HN comment previews (unique, AutoCLI can't do)
- LinkedIn feed (unique, AutoCLI doesn't support)
- Chinese finance/news (15+ sites AutoCLI doesn't have)
- Niche aggregators (Tildes, Slashdot, Douban)

**Where AutoCLI wins:**
- Speed (APIs vs DOM scraping)
- Reliability (structured data vs parsing)
- Mode count (more options per site)

**Our response:**
- Don't compete on speed/reliability for API-available sites
- Double down on unique data (comment previews, full body extraction)
- Keep expanding Chinese finance/news coverage
- Maintain recipe quality (rich previews, not just titles)
