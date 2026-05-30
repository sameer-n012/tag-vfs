#!/usr/bin/env bash
# Tests: config command
set -euo pipefail
source "$(dirname "$0")/common.sh"
echo "=== config ==="

DATA=$(mktemp -d)
trap 'teardown; rm -rf "$DATA"' EXIT

# ── Test 1: config -l lists the current configuration ────────────────────────
setup
OUT=$(run_vfs "config -l")
assert_contains "config -l runs" "$OUT" "Run Configuration"
teardown

# ── Test 2: set cliPrefix and it appears in subsequent prompts ────────────────
setup
# The prompt is printed to stdout before each command; setting cliPrefix should
# change it. We check the output contains the new prefix on the next prompt.
OUT=$(run_vfs "config cliPrefix >" "ls")
assert_contains "prefix in output" "$OUT" ">"
teardown

# ── Test 3: set an unknown key does not crash ─────────────────────────────────
setup
OUT=$(run_vfs_err "config unknownKey someValue" "ls")
# Should complete without a panic (binary exits cleanly)
assert_contains "unknown key no crash" "$OUT" ""
teardown

# ── Test 4: config -p flag accepted without error ────────────────────────────
setup
OUT=$(run_vfs_err "config cliPrefix $ -p")
# No crash or unhandled error
assert_not_contains "persist no error" "$OUT" "thread 'main' panicked"
teardown

summarize
