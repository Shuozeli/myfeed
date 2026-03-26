#!/bin/bash
# Run myfeed daemon and keep timestamped copies of the Atom feed.
# Each time feed.xml is updated, a copy is saved with a timestamp.
#
# Usage: ./scripts/run-with-changelog.sh

set -euo pipefail

FEED_DIR="docs/private/feeds"
FEED_FILE="docs/private/feed.xml"
mkdir -p "$FEED_DIR"

# Start myfeed in background
echo "Starting myfeed daemon..."
./target/release/myfeed run &
MYFEED_PID=$!
trap "kill $MYFEED_PID 2>/dev/null; echo 'myfeed stopped'; exit" INT TERM

echo "myfeed PID: $MYFEED_PID"
echo "Feed changelog: $FEED_DIR/"
echo "Press Ctrl+C to stop"

# Watch for feed.xml changes and copy with timestamp
LAST_HASH=""
while kill -0 $MYFEED_PID 2>/dev/null; do
    if [ -f "$FEED_FILE" ]; then
        HASH=$(md5sum "$FEED_FILE" | cut -d' ' -f1)
        if [ "$HASH" != "$LAST_HASH" ]; then
            TIMESTAMP=$(date -u +"%Y%m%d-%H%M%S")
            cp "$FEED_FILE" "$FEED_DIR/feed-${TIMESTAMP}.xml"
            ITEMS=$(grep -c '<entry>' "$FEED_FILE" 2>/dev/null || echo 0)
            echo "[$(date -u +%H:%M:%S)] Feed updated: $ITEMS items -> feed-${TIMESTAMP}.xml"
            LAST_HASH="$HASH"
        fi
    fi
    sleep 30
done

echo "myfeed exited"
