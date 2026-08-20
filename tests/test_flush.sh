#!/usr/bin/env bash
# Tests: flush and open commands
# The flush test that requires modifying a cached file uses a named pipe
# so the test script can interleave with the running binary.
set -euo pipefail
source "$(dirname "$0")/common.sh"
echo "=== flush ==="

DATA=$(mktemp -d)
trap 'teardown; rm -rf "$DATA"' EXIT

# ── Test 1: flush with no cached files reports an error ───────────────────────
setup
printf '%s' "data" > "$DATA/f.txt"
run_vfs "import $DATA/f.txt" > /dev/null
OUT=$(run_vfs_err "flush -f f.txt")
assert_contains "no cache error" "$OUT" "Error"
teardown

# ── Test 2: open caches the file; flush reports no changes on first flush ──────
setup
printf '%s' "original" > "$DATA/orig.txt"
run_vfs "import $DATA/orig.txt" > /dev/null
OUT=$(run_vfs "open orig.txt" "flush -f orig.txt")
assert_contains "no changes on clean flush" "$OUT" "No changes"
teardown

# ── Test 3: open → modify cached copy → flush → content updated in archive ────
# Uses a named pipe so the test can write to the cache between open and flush.
setup
printf '%s' "original content" > "$DATA/edit.txt"
run_vfs "import $DATA/edit.txt" > /dev/null

FIFO=$(mktemp -u)
mkfifo "$FIFO"
OUT_FILE=$(mktemp)
"$BINARY" --home "$TEST_HOME" < "$FIFO" > "$OUT_FILE" 2>/dev/null &
BIN_PID=$!
exec 3>"$FIFO"

echo "open edit.txt" >&3
sleep 0.3   # allow caching to complete

CACHED=$(find "$TEST_HOME" -name "edit.txt" 2>/dev/null | head -1)
if [ -n "$CACHED" ]; then
    printf '%s' "modified content" > "$CACHED"
fi

echo "flush -f edit.txt" >&3
exec 3>&-
wait "$BIN_PID"
rm -f "$FIFO" "$OUT_FILE"

DEST=$(mktemp -d)
run_vfs "expand $DEST" > /dev/null
assert_file_content "flush updated content" "$DEST/edit.txt" "modified content"
rm -rf "$DEST"
teardown

# ── Test 4: flush -a flushes all cached files ─────────────────────────────────
setup
printf '%s' "aa" > "$DATA/aa.txt"
printf '%s' "bb" > "$DATA/bb.txt"
run_vfs "import $DATA/aa.txt" "import $DATA/bb.txt" > /dev/null
OUT=$(run_vfs "open aa.txt" "open bb.txt" "flush -a")
assert_contains "flush -a aa" "$OUT" "aa.txt"
assert_contains "flush -a bb" "$OUT" "bb.txt"
teardown

summarize
