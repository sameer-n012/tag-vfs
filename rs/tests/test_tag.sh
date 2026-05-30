#!/usr/bin/env bash
# Tests: tag add/remove, filtering by tag
set -euo pipefail
source "$(dirname "$0")/common.sh"
echo "=== tag ==="

DATA=$(mktemp -d)
trap 'teardown; rm -rf "$DATA"' EXIT

# ── Test 1: add a tag and filter ls by it ─────────────────────────────────────
setup
printf '%s' "work"     > "$DATA/report.txt"
printf '%s' "personal" > "$DATA/diary.txt"
run_vfs "import $DATA/report.txt" "import $DATA/diary.txt" \
        "tag -f report.txt -t work" > /dev/null
OUT=$(run_vfs "ls work")
assert_contains     "tagged in ls"     "$OUT" "report.txt"
assert_not_contains "untagged not in ls" "$OUT" "diary.txt"
teardown

# ── Test 2: remove a tag ──────────────────────────────────────────────────────
setup
printf '%s' "data" > "$DATA/file.txt"
run_vfs "import $DATA/file.txt" "tag -f file.txt -t mytag" > /dev/null
run_vfs "tag -f file.txt -t mytag -d" > /dev/null
OUT=$(run_vfs "ls mytag")
assert_not_contains "tag removed" "$OUT" "file.txt"
teardown

# ── Test 3: multiple tags on one file, AND semantics for ls ───────────────────
setup
printf '%s' "multi" > "$DATA/multi.txt"
printf '%s' "solo"  > "$DATA/solo.txt"
run_vfs "import $DATA/multi.txt" "import $DATA/solo.txt" \
        "tag -f multi.txt -t alpha" \
        "tag -f multi.txt -t beta" \
        "tag -f solo.txt  -t alpha" > /dev/null
OUT_A=$(run_vfs "ls alpha")
OUT_B=$(run_vfs "ls beta")
OUT_AB=$(run_vfs "ls alpha beta")
assert_contains     "ls alpha has multi"  "$OUT_A"  "multi.txt"
assert_contains     "ls alpha has solo"   "$OUT_A"  "solo.txt"
assert_contains     "ls beta has multi"   "$OUT_B"  "multi.txt"
assert_not_contains "ls beta no solo"     "$OUT_B"  "solo.txt"
assert_contains     "ls both has multi"   "$OUT_AB" "multi.txt"
assert_not_contains "ls both no solo"     "$OUT_AB" "solo.txt"
teardown

summarize
