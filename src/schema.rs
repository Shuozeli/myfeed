// @generated automatically by Diesel CLI.

diesel::table! {
    crawl_snapshots (id) {
        id -> Integer,
        site -> Text,
        crawled_at -> Text,
        item_count -> Integer,
        items_json -> Text,
    }
}

diesel::table! {
    event_log (id) {
        id -> Integer,
        event_type -> Text,
        site -> Text,
        details -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    feed_items (id) {
        id -> Integer,
        site -> Text,
        external_id -> Text,
        title -> Text,
        url -> Text,
        preview -> Text,
        raw_json -> Text,
        created_at -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(crawl_snapshots, event_log, feed_items,);
