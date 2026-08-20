#!/usr/bin/env bash
# Tests: merge command (merge another archive into the working one)
set -euo pipefail
source "$(dirname "$0")/common.sh"
echo "=== merge ==="

DATA=$(mktemp -d)
trap 'teardown; rm -rf "$DATA"' EXIT

# ── Test 1: files from both archives are present after merge ──────────────────
setup
ARCHIVE_A="$TEST_HOME"
ARCHIVE_B=$(mktemp -d)
printf '%s' "from A" > "$DATA/file_a.txt"
printf '%s' "from B" > "$DATA/file_b.txt"
printf "import $DATA/file_a.txt\n" | "$BINARY" --home "$ARCHIVE_A" > /dev/null 2>&1
printf "import $DATA/file_b.txt\n" | "$BINARY" --home "$ARCHIVE_B" > /dev/null 2>&1
run_vfs "merge $ARCHIVE_B/archive.dat" > /dev/null
OUT=$(run_vfs "ls")
assert_contains "merge has A" "$OUT" "file_a.txt"
assert_contains "merge has B" "$OUT" "file_b.txt"
rm -rf "$ARCHIVE_B"
teardown

# ── Test 2: content of merged files is correct ────────────────────────────────
setup
ARCHIVE_B=$(mktemp -d)
printf '%s' "content from A" > "$DATA/ca.txt"
printf '%s' "content from B" > "$DATA/cb.txt"
printf "import $DATA/ca.txt\n" | "$BINARY" --home "$TEST_HOME" > /dev/null 2>&1
printf "import $DATA/cb.txt\n" | "$BINARY" --home "$ARCHIVE_B" > /dev/null 2>&1
run_vfs "merge $ARCHIVE_B/archive.dat" > /dev/null
DEST=$(mktemp -d)
run_vfs "expand $DEST" > /dev/null
assert_file_content "merge content A" "$DEST/ca.txt" "content from A"
assert_file_content "merge content B" "$DEST/cb.txt" "content from B"
rm -rf "$DEST" "$ARCHIVE_B"
teardown

# ── Test 3: tags from merged archive are preserved ────────────────────────────
setup
ARCHIVE_B=$(mktemp -d)
printf '%s' "tagged in B" > "$DATA/tagged_b.txt"
printf "import $DATA/tagged_b.txt\ntag -f tagged_b.txt -t btag\n" | \
    "$BINARY" --home "$ARCHIVE_B" > /dev/null 2>&1
run_vfs "merge $ARCHIVE_B/archive.dat" > /dev/null
OUT=$(run_vfs "ls btag")
assert_contains "merged tag preserved" "$OUT" "tagged_b.txt"
rm -rf "$ARCHIVE_B"
teardown

summarize
