# Agent Digest Guide

This guide is for AI agents (Claude Code, Gemini CLI, etc.) that want
to read and summarize myfeed data.

## Tiered Reading (recommended)

Feed data can be large (200+ items/day). Use a two-step approach to
stay within context limits:

### Step 1: Read the index (~2K tokens for 200 items)

```bash
myfeed dump --hours 24 --compact
```

Returns a compact index with `id`, `site`, `title`, `url` per item.
Scan titles to decide which items are worth reading in full.

### Step 2: Fetch details for interesting items

```bash
myfeed dump --ids 42,55,78,91,103
```

Returns full details (`preview`, `raw_json`, `created_at`) for the
specified item IDs. Read the preview text to produce your summary.

### Example workflow

```
Agent: runs `myfeed dump --hours 24 --compact`
       -> sees 200 items, scans titles
       -> picks 15 interesting items by ID

Agent: runs `myfeed dump --ids 12,45,67,89,102,115,130,142,155,167,180,190,195,198,200`
       -> reads full preview text for those 15 items
       -> produces a digest
```

This keeps total token usage under ~5K regardless of how many items
are in the database.

## Other Commands

```bash
# Full dump (all fields, can be large)
myfeed dump --hours 6

# Filter by site
myfeed dump --hours 24 --compact --site hackernews --site reddit

# Recent items as text table
myfeed list -l 50

# Crawl snapshots
myfeed snapshots hackernews
myfeed snapshot 42

# Event log
myfeed events -l 20
```

## Output Modes

| Command | Mode | Token cost | Use case |
|---------|------|-----------|----------|
| `dump --compact` | `index` | ~10 tokens/item | Scan titles, pick items |
| `dump --ids X,Y` | `detail` | ~200 tokens/item | Read full content |
| `dump` (default) | `full` | ~200 tokens/item | Small time ranges only |

## Interpreting Per-Site Fields

Items have common fields (`site`, `title`, `url`, `preview`, `created_at`).
The `raw_json` field (available in detail mode) contains per-site metadata:

| Site | Key signals |
|------|------------|
| hackernews | `score` (points), `comments` (count) -- higher = more interest |
| reddit | `upvotes`, `comments`, `subreddit` -- subreddit gives context |
| zhihu | `upvotes`, `answers`, `topic` -- topic tags useful for categorization |
| x | `likes`, `retweets`, `replies`, `author` -- engagement shows importance |
| linkedin | `likes`, `comments`, `author`, `company` |
| xueqiu | `likes`, `replies`, `symbol` (stock ticker) |
| 1point3acres | `post_content` (full text), `forum` section |

## Prompt Templates

Pre-written prompts are in `prompts/`:

- `daily-digest.md` -- Top stories + topic clusters, under 500 words
- `trending-topics.md` -- Cross-site trending topics
- `tech-radar.md` -- Tech-focused summary (tools, papers, career)

Read a template, then apply it to the dump data.

## Tips for Good Digests

- **Cross-reference sites.** The same story on HN and Reddit is more
  significant than one that appears on only one site.
- **Use engagement metrics** from `raw_json`. HN score > 200 or Reddit
  upvotes > 1000 suggests wide interest.
- **Group by topic, not site.** "AI developments" is more useful than
  "here's what was on HN."
- **Translate Chinese items.** Zhihu, Xueqiu, and 1point3acres items
  are in Chinese. Translate or summarize for English-speaking users.
