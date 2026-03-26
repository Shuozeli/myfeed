# Trending Topics

Read the feed data from `myfeed dump --hours 12` and identify what
topics are trending across sites.

## Output Format

List 5-10 topics, each with:
- Topic name (2-4 words)
- Which sites it appeared on
- Brief description (1 sentence)
- Representative URL

## Rules

- A topic is "trending" if it appears on 2+ sites, or has unusually
  high engagement on one site (HN score > 200, Reddit upvotes > 500).
- Prioritize topics that cross language boundaries (e.g., same tech
  story on HN and Zhihu).
- Ignore routine content (daily discussion threads, recurring posts).
