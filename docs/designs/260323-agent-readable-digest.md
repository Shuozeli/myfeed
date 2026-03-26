# Agent-Readable Feed Digest

## Problem

Users want AI-powered summaries of their feeds -- trending topics,
cross-site patterns, highlights. But embedding LLM calls into myfeed
adds complexity, cost coupling, and a runtime dependency on external
APIs.

## Approach

Keep myfeed dumb. Make the data agent-readable. Let external agents
(Claude Code, Gemini CLI, etc.) read the data and produce digests
on demand.

This follows the same philosophy as recipes: intelligence at authoring
time (the agent), mechanical execution at runtime (the tool).

## Design

### 1. `myfeed dump` CLI command

A new command that outputs recent feed items as clean JSON to stdout,
optimized for agent consumption:

```bash
# Last 6 hours, all sites
myfeed dump --hours 6

# Last 24 hours, specific sites
myfeed dump --hours 24 --site hackernews --site reddit

# Compact mode (title + url only, no preview)
myfeed dump --hours 6 --compact
```

Output format (one JSON object, not JSONL):

```json
{
  "period": "2026-03-23T00:00:00Z to 2026-03-23T06:00:00Z",
  "total_items": 85,
  "sites": {
    "hackernews": 30,
    "reddit": 20,
    "zhihu": 15,
    "x": 10,
    "1point3acres": 10
  },
  "items": [
    {
      "site": "hackernews",
      "title": "Flash-MoE: Running a 397B Parameter Model on a Laptop",
      "url": "https://...",
      "preview": "...",
      "score": 241,
      "comments": 140
    }
  ]
}
```

This is pure data extraction -- no LLM, no summarization.

### 2. Agent guide (`docs/agent-digest-guide.md`)

A guide that agents read to understand how to produce digests:

- Where the data lives (`myfeed dump`, `myfeed list`, feed.xml)
- How to interpret per-site fields (score = importance on HN,
  upvotes = importance on Reddit, etc.)
- Prompt templates for different digest styles (quick summary,
  deep analysis, cross-site trends)
- How to output (Telegram via `myfeed notify`, or just print)

### 3. Prompt templates

Pre-written prompts stored in `prompts/` that agents can use:

```
prompts/
  daily-digest.md      # "Summarize the top stories from the last 24h"
  trending-topics.md   # "What topics appear across multiple sites?"
  tech-radar.md        # "What new technologies are being discussed?"
```

These are plain markdown files with instructions. The agent reads
them, reads the feed data, and produces the summary.

### 4. Example agent workflow

```bash
# User asks Claude Code for a digest
> summarize my feeds from today

# Claude Code runs:
myfeed dump --hours 24

# Reads the JSON, applies the daily-digest prompt, produces:
"Today across your feeds:
 - AI: Flash-MoE paper trending on HN (241 points), Reddit discussing
   AI layoffs tracker...
 - Finance: Danone acquiring Huel for $1.2B (X), tariff impact on
   households (1point3acres)..."
```

## Why This is Better Than Embedded LLM Calls

- **No API key in myfeed.** No cost coupling, no rate limits.
- **Model-agnostic.** Works with Claude, Gemini, GPT, local models.
- **Agent controls the prompt.** Different agents can produce different
  digest styles from the same data.
- **Cacheable.** `myfeed dump` output is deterministic. Agent can
  cache and diff.
- **Testable.** The dump command is a pure data query. The digest
  quality depends on the agent, not myfeed.

## Implementation

- `myfeed dump` command: query `feed_items` by time range, output JSON
- `docs/agent-digest-guide.md`: instructions for agents
- `prompts/*.md`: reusable prompt templates
- No new dependencies, no LLM integration in Rust code
