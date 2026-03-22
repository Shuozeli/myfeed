CREATE TABLE crawl_snapshots (
    id          INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    site        TEXT NOT NULL,
    crawled_at  TEXT NOT NULL,
    item_count  INTEGER NOT NULL DEFAULT 0,
    items_json  TEXT NOT NULL DEFAULT '[]'
);

CREATE INDEX idx_crawl_snapshots_site_time ON crawl_snapshots (site, crawled_at DESC);
