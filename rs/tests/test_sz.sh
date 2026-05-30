#!/usr/bin/env bash
# Tests: sz (size) command
set -euo pipefail
source "$(dirname "$0")/common.sh"
echo "=== sz ==="

DATA=$(mktemp -d)
trap 'teardown; rm -rf "$DATA"' EXIT

# ── Test 1: size of a known file ──────────────────────────────────────────────
setup
printf '%s' "hello" > "$DATA/five.txt"   # exactly 5 bytes
run_vfs "import $DATA/five.txt" > /dev/null
OUT=$(run_vfs "sz")
assert_contains "sz 5 bytes" "$OUT" "5.0 B"
teardown

# ── Test 2: total size across multiple files ──────────────────────────────────
setup
printf '%s' "aaaaa" > "$DATA/five.txt"   # 5 bytes
printf '%s' "bbbbb" > "$DATA/also5.txt"  # 5 bytes
run_vfs "import $DATA/five.txt" "import $DATA/also5.txt" > /dev/null
OUT=$(run_vfs "sz")
assert_contains "sz 10 bytes" "$OUT" "10.0 B"
teardown

# ── Test 3: size with tag filter ──────────────────────────────────────────────
setup
printf '%s' "hello" > "$DATA/tagged.txt"     # 5 bytes, will be tagged
printf '%s' "ignored" > "$DATA/untagged.txt" # 7 bytes, untagged
run_vfs "import $DATA/tagged.txt" "import $DATA/untagged.txt" \
        "tag -f tagged.txt -t counted" > /dev/null
OUT=$(run_vfs "sz counted")
assert_contains     "sz tag only"  "$OUT" "5.0 B"
assert_not_contains "sz excludes"  "$OUT" "12.0 B"
teardown

summarize
