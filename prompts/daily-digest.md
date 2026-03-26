# Daily Digest

Read the feed data from `myfeed dump --hours 24` and produce a concise
daily summary.

## Output Format

**Top Stories** (3-5 items that had the highest engagement or appeared
on multiple sites)

**By Topic** (group remaining items into 3-5 topic clusters, e.g.,
"AI/ML", "Career/Immigration", "Markets", "Tech Industry", "Misc")

Each topic: 2-3 sentence summary of what's being discussed, with
links to the most relevant items.

**Quick Stats**: total items, which sites were most active, any notable
trends compared to previous days.

## Rules

- Lead with the most important stories, not chronological order.
- If an item appears on multiple sites, mention that.
- Translate Chinese-language items (Zhihu, Xueqiu, 1point3acres) to
  English in the summary.
- Keep the total digest under 500 words.
- Use plain text, no markdown formatting.
