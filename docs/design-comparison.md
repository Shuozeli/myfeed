# Recipe-Based vs Runtime Browser Agents

## Overview

Several tools let AI agents control browsers at runtime -- the agent
sees a page and decides what to do next. pwright takes a different
approach: site logic is authored as YAML recipes ahead of time and
executed mechanically. No LLM at runtime.

These are complementary approaches for different use cases.

## Landscape

### Runtime agent tools

[**Page Agent**](https://github.com/alibaba/page-agent) (Alibaba) --
In-page JS library. LLM receives a simplified text DOM (interactive
elements indexed as [0], [1], etc.) and decides which to click/fill/scroll.
ReAct loop: observe, reason, act, repeat. Max 40 steps per task. No
screenshots needed -- text-only DOM representation.

[**browser-use**](https://github.com/browser-use/browser-use) -- Python
library. LLM controls a Playwright browser via CDP. Uses screenshots +
DOM extraction. Agent sees the page visually and decides actions.

[**PinchTab**](https://github.com/pinchtab/pinchtab) -- Go HTTP server
that sits between agents and Chrome via CDP. Agents talk HTTP, PinchTab
translates to CDP. Uses accessibility-tree snapshots with stable element
refs (not CSS selectors). Token-efficient text extraction (~800 tokens/page
vs 10K+ for screenshots). Built-in security layer: token auth, domain
allowlist, prompt-injection detection. The agent still decides actions
at runtime, but PinchTab adds a protocol-level control plane with
persistent profiles for auth state.

[**OpenClaw**](https://github.com/openclaw/openclaw) -- Similar pattern.
LLM drives a browser session, making runtime decisions about navigation
and interaction.

All follow the same model: **LLM decides actions at runtime** based on
current page state.

### Recipe tools

[**pwright**](https://github.com/shuozeli/pwright) -- Rust CDP bridge
with YAML recipe system. Site-specific logic (selectors, JS extraction)
is authored once and committed. Runtime execution is sequential step
execution with no LLM involvement.

## The Core Difference

Runtime agents put intelligence at execution time. The LLM adapts to
whatever page it sees. Each step costs LLM tokens and introduces
non-determinism.

Recipes put intelligence at authoring time. A human (with AI help)
figures out the page once, writes a recipe, tests it. Runtime is
mechanical.

## Trade-offs

| | Runtime agents | PinchTab | Recipes |
|---|---|---|---|
| Flexibility | High | High | Needs a recipe per site |
| Setup cost | Near zero | Near zero | One-time per site |
| Per-run cost | ~3-8K tokens/step | ~800 tokens/step | Zero |
| Reproducibility | Non-deterministic | Non-deterministic | Deterministic |
| Security | Varies by tool | Token auth + domain allowlist + IDPI | Bounded to recipe steps |
| Element stability | CSS selectors (fragile) | Accessibility refs (stable) | CSS selectors (fragile) |
| Site changes | May adapt | May adapt | Manual recipe update |

## When to Use Which

**Recipes** -- recurring tasks on a schedule, unattended automation on
logged-in accounts, cost-sensitive workloads at scale.

**PinchTab** -- agent-driven automation where you want a security layer,
token efficiency, and stable element refs. Good middle ground when you
need runtime intelligence but want guardrails.

**Page Agent / browser-use** -- one-off exploration, dynamic workflows,
SaaS copilots, understanding a new site before writing a recipe.

They work well together: use runtime agents during recipe authoring
(explore the DOM, understand the page), then recipes for execution.
