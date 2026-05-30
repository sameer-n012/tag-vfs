#!/usr/bin/env bash
# Tests: remove command
set -euo pipefail
source "$(dirname "$0")/common.sh"
echo "=== remove ==="

DATA=$(mktemp -d)
trap 'teardown; rm -rf "$DATA"' EXIT

# ── Test 1: remove a file by name ─────────────────────────────────────────────
setup
printf '%s' "keep"   > "$DATA/keep.txt"
printf '%s' "delete" > "$DATA/delete.txt"
run_vfs "import $DATA/keep.txt" "import $DATA/delete.txt" > /dev/null
OUT=$(run_vfs "remove -f delete.txt" "ls")
assert_not_contains "removed gone"    "$OUT" "delete.txt"
assert_contains     "kept remains"    "$OUT" "keep.txt"
teardown

# ── Test 2: remove files by tag ───────────────────────────────────────────────
setup
printf '%s' "work file"     > "$DATA/report.txt"
printf '%s' "personal file" > "$DATA/journal.txt"
run_vfs "import $DATA/report.txt" "import $DATA/journal.txt" \
        "tag -f report.txt -t work" > /dev/null
OUT=$(run_vfs "remove -t work" "ls")
assert_not_contains "tagged removed" "$OUT" "report.txt"
assert_contains     "untagged kept"  "$OUT" "journal.txt"
teardown

# ── Test 3: removing a non-existent file does not crash ───────────────────────
setup
printf '%s' "file" > "$DATA/present.txt"
run_vfs "import $DATA/present.txt" > /dev/null
OUT=$(run_vfs_err "remove -f ghost.txt" "ls")
assert_contains "present after ghost remove" "$OUT" "present.txt"
teardown

summarize
