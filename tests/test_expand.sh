#!/usr/bin/env bash
# Tests: expand and expand_from commands
set -euo pipefail
source "$(dirname "$0")/common.sh"
echo "=== expand ==="

DATA=$(mktemp -d)
trap 'teardown; rm -rf "$DATA"' EXIT

# ── Test 1: expand writes files to destination directory ──────────────────────
setup
printf '%s' "hello world" > "$DATA/doc.txt"
run_vfs "import $DATA/doc.txt" > /dev/null
DEST=$(mktemp -d)
run_vfs "expand $DEST" > /dev/null
assert_file_content "expand content" "$DEST/doc.txt" "hello world"
rm -rf "$DEST"
teardown

# ── Test 2: expand multiple files ─────────────────────────────────────────────
setup
printf '%s' "one"   > "$DATA/one.txt"
printf '%s' "two"   > "$DATA/two.txt"
printf '%s' "three" > "$DATA/three.txt"
run_vfs "import $DATA/one.txt" "import $DATA/two.txt" "import $DATA/three.txt" > /dev/null
DEST=$(mktemp -d)
run_vfs "expand $DEST" > /dev/null
assert_dir_contains_file "expand multi one"   "$DEST" "one.txt"
assert_dir_contains_file "expand multi two"   "$DEST" "two.txt"
assert_dir_contains_file "expand multi three" "$DEST" "three.txt"
assert_file_content "expand multi content one"   "$DEST/one.txt"   "one"
assert_file_content "expand multi content two"   "$DEST/two.txt"   "two"
assert_file_content "expand multi content three" "$DEST/three.txt" "three"
rm -rf "$DEST"
teardown

# ── Test 3: expand -f expands a specific archive file ─────────────────────────
setup
printf '%s' "from alt archive" > "$DATA/alt.txt"
run_vfs "import $DATA/alt.txt" > /dev/null
DEST=$(mktemp -d)
ARCHIVE_PATH="$TEST_HOME/archive.dat"
# Open a fresh home for expand_from so we don't overwrite the first archive
OTHER_HOME=$(mktemp -d)
printf 'expand %s -f %s\n' "$DEST" "$ARCHIVE_PATH" | \
    "$BINARY" --home "$OTHER_HOME" > /dev/null 2>&1
assert_dir_contains_file "expand_from" "$DEST" "alt.txt"
assert_file_content "expand_from content" "$DEST/alt.txt" "from alt archive"
rm -rf "$DEST" "$OTHER_HOME"
teardown

summarize
