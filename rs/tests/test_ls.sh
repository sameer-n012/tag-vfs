#!/usr/bin/env bash
# Tests: ls command (list files, tag filters)
set -euo pipefail
source "$(dirname "$0")/common.sh"
echo "=== ls ==="

DATA=$(mktemp -d)
trap 'teardown; rm -rf "$DATA"' EXIT

# ── Test 1: ls with no args shows all files ───────────────────────────────────
setup
printf '%s' "a" > "$DATA/a.txt"
printf '%s' "b" > "$DATA/b.txt"
printf '%s' "c" > "$DATA/c.txt"
run_vfs "import $DATA/a.txt" "import $DATA/b.txt" "import $DATA/c.txt" > /dev/null
OUT=$(run_vfs "ls")
assert_contains "ls all a" "$OUT" "a.txt"
assert_contains "ls all b" "$OUT" "b.txt"
assert_contains "ls all c" "$OUT" "c.txt"
teardown

# ── Test 2: ls with tag shows only matching files ─────────────────────────────
setup
printf '%s' "tagged"   > "$DATA/tagged.txt"
printf '%s' "untagged" > "$DATA/untagged.txt"
run_vfs "import $DATA/tagged.txt" "import $DATA/untagged.txt" \
        "tag -f tagged.txt -t selected" > /dev/null
OUT=$(run_vfs "ls selected")
assert_contains     "ls tag match"     "$OUT" "tagged.txt"
assert_not_contains "ls tag no match"  "$OUT" "untagged.txt"
teardown

# ── Test 3: ls on empty archive shows nothing ─────────────────────────────────
setup
OUT=$(run_vfs "ls")
assert_not_contains "ls empty" "$OUT" ".txt"
teardown

# ── Test 4: ls positional tags (no -t flag) ───────────────────────────────────
setup
printf '%s' "x" > "$DATA/x.txt"
run_vfs "import $DATA/x.txt" "tag -f x.txt -t pos" > /dev/null
OUT=$(run_vfs "ls pos")
assert_contains "ls positional tag" "$OUT" "x.txt"
teardown

summarize
