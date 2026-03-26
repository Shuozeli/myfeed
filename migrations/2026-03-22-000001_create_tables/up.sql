CREATE TABLE feed_items (
    id          INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    site        TEXT NOT NULL,
    external_id TEXT NOT NULL,
    title       TEXT NOT NULL,
    url         TEXT NOT NULL,
    preview     TEXT NOT NULL DEFAULT '',
    raw_json    TEXT NOT NULL DEFAULT '{}',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),

    UNIQUE (site, external_id)
);

CREATE INDEX idx_feed_items_site_created ON feed_items (site, created_at DESC);

CREATE TABLE event_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    event_type  TEXT NOT NULL,
    site        TEXT NOT NULL,
    details     TEXT NOT NULL DEFAULT '{}',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX idx_event_log_created ON event_log (created_at DESC);

CREATE TABLE crawl_snapshots (
    id          INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    site        TEXT NOT NULL,
    crawled_at  TEXT NOT NULL,
    item_count  INTEGER NOT NULL DEFAULT 0,
    items_json  TEXT NOT NULL DEFAULT '[]'
);

CREATE INDEX idx_crawl_snapshots_site_time ON crawl_snapshots (site, crawled_at DESC);
