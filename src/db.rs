use chrono::Utc;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::sync::Mutex;

use crate::proto;
use crate::schema::{crawl_snapshots, event_log, feed_items};

/// A feed item stored in the database for deduplication and history.
#[derive(Debug, Clone, Queryable, Selectable, serde::Serialize)]
#[diesel(table_name = feed_items)]
pub struct FeedItem {
    pub id: i32,
    pub site: String,
    pub external_id: String,
    pub title: String,
    pub url: String,
    pub preview: String,
    pub raw_json: String,
    pub created_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = feed_items)]
struct NewFeedItem<'a> {
    pub site: &'a str,
    pub external_id: &'a str,
    pub title: &'a str,
    pub url: &'a str,
    pub preview: &'a str,
    pub raw_json: &'a str,
    pub created_at: &'a str,
}

#[derive(Debug, Queryable, Selectable, serde::Serialize)]
#[diesel(table_name = event_log)]
pub struct EventLogEntry {
    pub id: i32,
    pub event_type: String,
    pub site: String,
    pub details: String,
    pub created_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = event_log)]
struct NewEventLog<'a> {
    pub event_type: &'a str,
    pub site: &'a str,
    pub details: &'a str,
    pub created_at: &'a str,
}

#[derive(Debug, Queryable, Selectable, serde::Serialize)]
#[diesel(table_name = crawl_snapshots)]
pub struct CrawlSnapshotRow {
    pub id: i32,
    pub site: String,
    pub crawled_at: String,
    pub item_count: i32,
    pub items_json: String,
}

#[derive(Insertable)]
#[diesel(table_name = crawl_snapshots)]
struct NewCrawlSnapshot<'a> {
    pub site: &'a str,
    pub crawled_at: &'a str,
    pub item_count: i32,
    pub items_json: &'a str,
}

/// Database operations for feed items. All queries run inside transactions.
/// Uses `Mutex` because SQLite connections are not `Send` across threads.
pub struct FeedDb {
    conn: Mutex<SqliteConnection>,
}

impl FeedDb {
    pub fn new(database_url: &str) -> Self {
        let conn = SqliteConnection::establish(database_url)
            .unwrap_or_else(|e| panic!("failed to connect to {database_url}: {e}"));
        Self {
            conn: Mutex::new(conn),
        }
    }

    /// Run embedded migrations.
    pub fn migrate(&self) {
        use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
        const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

        let mut conn = self.conn.lock().expect("db lock poisoned");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("failed to run migrations");
    }

    /// Insert a feed item (ignore if duplicate). Always succeeds for snapshots.
    pub fn insert_item(
        &self,
        site: &str,
        external_id: &str,
        title: &str,
        url: &str,
        preview: &str,
        raw_json: &serde_json::Value,
    ) -> Result<(), diesel::result::Error> {
        let mut conn = self.conn.lock().expect("db lock poisoned");
        let now = Utc::now().to_rfc3339();
        let json_str =
            serde_json::to_string(raw_json).expect("serializing serde_json::Value is infallible");

        conn.transaction(|conn| {
            diesel::insert_or_ignore_into(feed_items::table)
                .values(NewFeedItem {
                    site,
                    external_id,
                    title,
                    url,
                    preview,
                    raw_json: &json_str,
                    created_at: &now,
                })
                .execute(conn)?;
            Ok(())
        })
    }

    /// Check if an item was seen within the dedup window.
    /// `window_hours = 0` means forever (any match returns true).
    /// Returns true if the item exists (should NOT notify).
    pub fn was_seen_recently(
        &self,
        site: &str,
        external_id: &str,
        window_hours: u64,
    ) -> Result<bool, diesel::result::Error> {
        let mut conn = self.conn.lock().expect("db lock poisoned");
        conn.transaction(|conn| {
            use diesel::dsl::exists;
            if window_hours == 0 {
                // Forever dedup: any match
                diesel::select(exists(
                    feed_items::table
                        .filter(feed_items::site.eq(site))
                        .filter(feed_items::external_id.eq(external_id)),
                ))
                .get_result(conn)
            } else {
                // Windowed dedup: only match within the window
                #[allow(clippy::cast_possible_wrap)] // window_hours is small (< 1000)
                let cutoff =
                    (Utc::now() - chrono::Duration::hours(window_hours as i64)).to_rfc3339();
                diesel::select(exists(
                    feed_items::table
                        .filter(feed_items::site.eq(site))
                        .filter(feed_items::external_id.eq(external_id))
                        .filter(feed_items::created_at.ge(&cutoff)),
                ))
                .get_result(conn)
            }
        })
    }

    /// Log an event to the `event_log` table.
    pub fn log_event(
        &self,
        event_type: &str,
        site: &str,
        details: &serde_json::Value,
    ) -> Result<(), diesel::result::Error> {
        let mut conn = self.conn.lock().expect("db lock poisoned");
        let now = Utc::now().to_rfc3339();
        let details_str =
            serde_json::to_string(details).expect("serializing serde_json::Value is infallible");

        conn.transaction(|conn| {
            diesel::insert_into(event_log::table)
                .values(NewEventLog {
                    event_type,
                    site,
                    details: &details_str,
                    created_at: &now,
                })
                .execute(conn)?;
            Ok(())
        })
    }

    /// Get recent feed items, optionally filtered by site.
    pub fn recent_items(
        &self,
        site_filter: Option<&str>,
        limit: i64,
    ) -> Result<Vec<FeedItem>, diesel::result::Error> {
        let mut conn = self.conn.lock().expect("db lock poisoned");
        conn.transaction(|conn| {
            let mut query = feed_items::table
                .order(feed_items::id.desc())
                .limit(limit)
                .into_boxed();
            if let Some(site) = site_filter {
                query = query.filter(feed_items::site.eq(site));
            }
            query.select(FeedItem::as_select()).load(conn)
        })
    }

    /// Get feed items since a given timestamp, optionally filtered by sites.
    pub fn items_since(
        &self,
        since: &str,
        sites: &[String],
    ) -> Result<Vec<FeedItem>, diesel::result::Error> {
        let mut conn = self.conn.lock().expect("db lock poisoned");
        conn.transaction(|conn| {
            let mut query = feed_items::table
                .filter(feed_items::created_at.ge(since))
                .order(feed_items::created_at.desc())
                .into_boxed();
            if !sites.is_empty() {
                query = query.filter(feed_items::site.eq_any(sites));
            }
            query.select(FeedItem::as_select()).load(conn)
        })
    }

    /// Get specific feed items by their IDs.
    pub fn items_by_ids(&self, ids: &[i32]) -> Result<Vec<FeedItem>, diesel::result::Error> {
        let mut conn = self.conn.lock().expect("db lock poisoned");
        conn.transaction(|conn| {
            feed_items::table
                .filter(feed_items::id.eq_any(ids))
                .select(FeedItem::as_select())
                .load(conn)
        })
    }

    /// Delete event log entries older than N days. Returns number of rows deleted.
    pub fn cleanup_old_events(&self, retention_days: i64) -> Result<usize, diesel::result::Error> {
        let mut conn = self.conn.lock().expect("db lock poisoned");
        let cutoff = (Utc::now() - chrono::Duration::days(retention_days)).to_rfc3339();
        conn.transaction(|conn| {
            diesel::delete(event_log::table.filter(event_log::created_at.lt(&cutoff))).execute(conn)
        })
    }

    /// Count consecutive `crawl_complete` events with 0 items for a site
    /// (most recent first). Stops counting at the first non-zero crawl.
    pub fn consecutive_empty_crawls(&self, site: &str) -> Result<i64, diesel::result::Error> {
        let mut conn = self.conn.lock().expect("db lock poisoned");
        conn.transaction(|conn| {
            let events: Vec<EventLogEntry> = event_log::table
                .filter(event_log::site.eq(site))
                .filter(event_log::event_type.eq("crawl_complete"))
                .order(event_log::id.desc())
                .limit(10)
                .select(EventLogEntry::as_select())
                .load(conn)?;

            let mut count = 0i64;
            for event in &events {
                if let Ok(details) =
                    serde_json::from_str::<crate::scheduler::CrawlCompleteEvent>(&event.details)
                {
                    if details.items_found == 0 {
                        count += 1;
                    } else {
                        break;
                    }
                }
            }
            Ok(count)
        })
    }

    /// Get recent events for debugging.
    pub fn recent_events(&self, limit: i64) -> Result<Vec<EventLogEntry>, diesel::result::Error> {
        let mut conn = self.conn.lock().expect("db lock poisoned");
        conn.transaction(|conn| {
            event_log::table
                .order(event_log::id.desc())
                .limit(limit)
                .select(EventLogEntry::as_select())
                .load(conn)
        })
    }

    /// Save a crawl snapshot (the full parsed JSON for a site at a point in time).
    pub fn save_snapshot(
        &self,
        snapshot: &proto::CrawlSnapshot,
    ) -> Result<(), diesel::result::Error> {
        let mut conn = self.conn.lock().expect("db lock poisoned");
        let items_json = serde_json::to_string(&snapshot.items)
            .expect("serializing proto FeedItems is infallible");

        conn.transaction(|conn| {
            diesel::insert_into(crawl_snapshots::table)
                .values(NewCrawlSnapshot {
                    site: &snapshot.site,
                    crawled_at: &snapshot.crawled_at,
                    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                    item_count: snapshot.items.len() as i32, // item counts are small (< 100)
                    items_json: &items_json,
                })
                .execute(conn)?;
            Ok(())
        })
    }

    /// Get recent snapshots for a site.
    pub fn recent_snapshots(
        &self,
        site: &str,
        limit: i64,
    ) -> Result<Vec<CrawlSnapshotRow>, diesel::result::Error> {
        let mut conn = self.conn.lock().expect("db lock poisoned");
        conn.transaction(|conn| {
            crawl_snapshots::table
                .filter(crawl_snapshots::site.eq(site))
                .order(crawl_snapshots::id.desc())
                .limit(limit)
                .select(CrawlSnapshotRow::as_select())
                .load(conn)
        })
    }

    /// Get a specific snapshot by ID.
    pub fn get_snapshot(
        &self,
        snapshot_id: i32,
    ) -> Result<Option<CrawlSnapshotRow>, diesel::result::Error> {
        let mut conn = self.conn.lock().expect("db lock poisoned");
        conn.transaction(|conn| {
            crawl_snapshots::table
                .find(snapshot_id)
                .select(CrawlSnapshotRow::as_select())
                .first(conn)
                .optional()
        })
    }
}
