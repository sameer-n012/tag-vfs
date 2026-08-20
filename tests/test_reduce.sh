#!/usr/bin/env bash
# Tests: reduce command (compress files into archive)
set -euo pipefail
source "$(dirname "$0")/common.sh"
echo "=== reduce ==="

DATA=$(mktemp -d)
trap 'teardown; rm -rf "$DATA"' EXIT

# ── Test 1: reduce a single file ──────────────────────────────────────────────
setup
printf '%s' "reduced content" > "$DATA/small.txt"
run_vfs "reduce $DATA/small.txt" > /dev/null
OUT=$(run_vfs "ls")
assert_contains "reduce single" "$OUT" "small.txt"
DEST=$(mktemp -d)
run_vfs "expand $DEST" > /dev/null
assert_file_content "reduce data" "$DEST/small.txt" "reduced content"
rm -rf "$DEST"
teardown

# ── Test 2: reduce a directory recursively ────────────────────────────────────
setup
SUBDIR="$DATA/subdir"
mkdir -p "$SUBDIR"
printf '%s' "file a" > "$SUBDIR/a.txt"
printf '%s' "file b" > "$SUBDIR/b.txt"
run_vfs "reduce $SUBDIR -r" > /dev/null
OUT=$(run_vfs "ls")
assert_contains "reduce dir a" "$OUT" "a.txt"
assert_contains "reduce dir b" "$OUT" "b.txt"
DEST=$(mktemp -d)
run_vfs "expand $DEST" > /dev/null
assert_file_content "reduce dir content a" "$DEST/a.txt" "file a"
assert_file_content "reduce dir content b" "$DEST/b.txt" "file b"
rm -rf "$DEST"
teardown

# ── Test 3: reduce multiple files accumulates in archive ──────────────────────
setup
printf '%s' "first"  > "$DATA/first.txt"
printf '%s' "second" > "$DATA/second.txt"
run_vfs "reduce $DATA/first.txt" "reduce $DATA/second.txt" > /dev/null
OUT=$(run_vfs "ls")
assert_contains "reduce accum first"  "$OUT" "first.txt"
assert_contains "reduce accum second" "$OUT" "second.txt"
teardown

summarize
