#!/usr/bin/env bash
# Tests: import command
set -euo pipefail
source "$(dirname "$0")/common.sh"
echo "=== import ==="

DATA=$(mktemp -d)
trap 'teardown; rm -rf "$DATA"' EXIT

# ── Test 1: import a single file, verify it appears in ls ─────────────────────
setup
printf '%s' "hello world" > "$DATA/note.txt"
OUT=$(run_vfs "import $DATA/note.txt" "ls")
assert_contains "single file" "$OUT" "note.txt"
teardown

# ── Test 2: import multiple files in one session ──────────────────────────────
setup
printf '%s' "alpha" > "$DATA/alpha.txt"
printf '%s' "beta"  > "$DATA/beta.txt"
OUT=$(run_vfs "import $DATA/alpha.txt" "import $DATA/beta.txt" "ls")
assert_contains "multi file a" "$OUT" "alpha.txt"
assert_contains "multi file b" "$OUT" "beta.txt"
teardown

# ── Test 3: recursive import of a directory ───────────────────────────────────
setup
SUBDIR="$DATA/subdir"
mkdir -p "$SUBDIR"
printf '%s' "one" > "$SUBDIR/one.txt"
printf '%s' "two" > "$SUBDIR/two.txt"
OUT=$(run_vfs "import $SUBDIR -r" "ls")
assert_contains "recursive a" "$OUT" "one.txt"
assert_contains "recursive b" "$OUT" "two.txt"
teardown

# ── Test 4: data integrity — content survives round-trip through the archive ──
setup
printf '%s' "exact content 12345" > "$DATA/exact.txt"
DEST=$(mktemp -d)
run_vfs "import $DATA/exact.txt" "expand $DEST" > /dev/null
assert_file_content "data integrity" "$DEST/exact.txt" "exact content 12345"
rm -rf "$DEST"
teardown

summarize
