# myfeed

[English](README.md) | 中文

针对不支持 RSS 的网站的替代方案。myfeed 连接到 Chrome 浏览器，通过 YAML recipe 提取帖子内容，并将新条目推送到 Telegram。

原理是通过 CDP 协议自动化一个真实的浏览器标签页，使用你已登录的 Chrome 会话，所以它能读取任何需要登录才能查看的页面。Recipe 是包含 JS 提取逻辑的 YAML 文件，添加一个新站点只需写一个文件，无需修改 Rust 代码。

```
Chrome (你的会话)  -->  YAML recipes  -->  SQLite 去重  -->  Telegram
```

## 工作原理

- 通过 CDP 连接 Chrome（使用你现有的会话和 cookies）
- 按计划运行 YAML recipe（默认每 30 分钟一次）
- 在 SQLite 中去重，将新条目推送到 Telegram
- Recipe 可由 AI agent 编写和维护，大多数几分钟即可完成
- 运行时无需消耗 LLM tokens，recipe 编写后执行是确定性的

## 已支持站点（23 个公开 recipe）

| 分类 | 站点 |
|------|------|
| 科技 | Hacker News, Reddit, V2EX, Slashdot, Tildes, InfoQ, GitHub Trending, Substack |
| 社交 | X (Twitter), LinkedIn, Telegram Channels, 豆瓣 |
| 财经 | 雪球, 东方财富, 富途, Finviz, Seeking Alpha |
| 中文 | 知乎, 一亩三分地, 微博热搜, 36氪 |

可自行添加私有 recipe（已 gitignore）来支持更多站点。

## 快速开始

```bash
# 1. 启动带远程调试的 Chrome
google-chrome --remote-debugging-port=9222 --user-data-dir=$HOME/.myfeed-chrome

# 2. 克隆并构建
git clone https://github.com/Shuozeli/myfeed.git && cd myfeed
cp .env.example .env   # 编辑填入你的 Telegram bot token 和 chat ID
cargo build --release

# 3. 登录站点（只需一次）
./target/release/myfeed login reddit
./target/release/myfeed login zhihu

# 4. 运行
./target/release/myfeed run   # 每 30 分钟抓取一次，新帖推送到 Telegram
```

## Recipe 示例

```yaml
# recipes/hackernews-feed.yaml
steps:
  - goto: "https://news.ycombinator.com"
    wait_for: ".athing"
  - eval:
      ref: extract_stories    # 返回 [{id, title, url, preview}] 的 JS 函数
      save_as: items
  - output:
      items: "{{ items }}"
```

每个 recipe 导航到页面，等待内容加载，运行 JS 提取条目，输出 JSON 数组。约定很简单：`{id, title, url, preview}`。

## 添加新站点

1. 创建 `recipes/<site>-feed.yaml`，包含提取 `[{id, title, url, preview}]` 的 JS
2. 将站点名称加入 `.env` 中的 `ENABLED_SITES`
3. 完成。无需修改 Rust 代码。

需要登录的站点，运行一次 `myfeed login <site>` 即可，会话 cookies 会保存在 Chrome 的 profile 目录中。

想请求支持新站点？[提交 issue](https://github.com/Shuozeli/myfeed/issues/new?template=new-site-recipe.yml)。

## 架构

```
src/
  main.rs        CLI: run, once, login, list, events, dump
  config.rs      所有配置来自环境变量（缺失则启动失败）
  crawler.rs     运行 pwright recipe，解析输出为 FeedItem
  scheduler.rs   异步循环：抓取 -> 快照 -> 去重 -> Telegram
  db.rs          SQLite via diesel，所有查询在事务中执行
  telegram.rs    消息队列，限速 1 msg/sec，429 退避
  feed.rs        生成 Atom 1.0 XML

recipes/         每个站点一个 YAML 文件，JS 提取逻辑，不涉及 Rust
proto/           Protobuf schema，每个站点有独立的 typed payload
```

## Agent 集成

`dump` 命令为 AI agent 提供 feed 数据：

```bash
myfeed dump --hours 24 --compact          # 扫描标题（约 10 tokens/条）
myfeed dump --ids 42,55,78                # 获取指定条目的完整详情
```

`prompts/` 目录下有日报、热门话题、技术雷达等 prompt 模板。详见 [agent digest guide](docs/agent-digest-guide.md)。

## 配置

| 变量 | 说明 |
|------|------|
| `CDP_ENDPOINT` | Chrome DevTools HTTP 地址（如 `http://localhost:9222`） |
| `DATABASE_URL` | SQLite 数据库路径（如 `myfeed.db`） |
| `TELEGRAM_BOT_TOKEN` | 从 [@BotFather](https://t.me/BotFather) 获取 |
| `TELEGRAM_CHAT_ID` | 目标聊天 ID |
| `CRAWL_INTERVAL_SECS` | 抓取间隔秒数（建议 `1800`） |
| `ENABLED_SITES` | 逗号分隔的站点名称 |
| `FILTER_KEYWORDS` | 可选：仅推送匹配的条目 |
| `DIGEST_MODE` | 可选：每个站点合并为一条消息 |
| `DEDUP_WINDOW_HOURS` | 可选：N 小时后允许重复推送（0 = 永不） |
| `FEED_OUTPUT_PATH` | 可选：输出 Atom feed XML |

所有必填变量缺失时直接 panic，不使用默认值。

## 依赖

基于 [pwright](https://github.com/shuozeli/pwright)（Chrome CDP 桥接 + recipe 引擎）、diesel（SQLite）、tokio、reqwest。

## 演示

我们在家庭服务器上运行 myfeed，每 30 分钟抓取 32 个站点（上述 23 个公开 recipe 加上新闻站点的私有 recipe）。完整周期约 6 分钟，每次通常发现 50-150 条新内容。

## 许可证

MIT
