#!/usr/bin/env bash
# Shared helpers for all bash integration tests.
# Source this file; do not run it directly.

BINARY="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/target/debug/tag-vfs"

PASS=0
FAIL=0

# Creates a fresh isolated home dir and sets TEST_HOME.
setup() {
    TEST_HOME=$(mktemp -d)
}

# Removes the isolated home dir.
teardown() {
    rm -rf "$TEST_HOME"
}

# Runs commands against the binary with the current TEST_HOME.
# Each argument is one REPL command. The session ends at EOF.
run_vfs() {
    printf '%s\n' "$@" | "$BINARY" --home "$TEST_HOME" 2>/dev/null
}

# Same as run_vfs but also captures stderr.
run_vfs_err() {
    printf '%s\n' "$@" | "$BINARY" --home "$TEST_HOME" 2>&1
}

assert_contains() {
    local label="$1" output="$2" needle="$3"
    if echo "$output" | grep -qF "$needle"; then
        echo "  PASS [$label]: output contains '$needle'"
        PASS=$((PASS + 1))
    else
        echo "  FAIL [$label]: expected '$needle' in output"
        echo "         got: $(echo "$output" | head -5)"
        FAIL=$((FAIL + 1))
    fi
}

assert_not_contains() {
    local label="$1" output="$2" needle="$3"
    if ! echo "$output" | grep -qF "$needle"; then
        echo "  PASS [$label]: output does not contain '$needle'"
        PASS=$((PASS + 1))
    else
        echo "  FAIL [$label]: did not expect '$needle' in output"
        FAIL=$((FAIL + 1))
    fi
}

assert_file_exists() {
    local label="$1" path="$2"
    if [ -f "$path" ]; then
        echo "  PASS [$label]: file exists: $(basename "$path")"
        PASS=$((PASS + 1))
    else
        echo "  FAIL [$label]: file not found: $path"
        FAIL=$((FAIL + 1))
    fi
}

assert_file_content() {
    local label="$1" path="$2" expected="$3"
    local actual
    actual=$(cat "$path" 2>/dev/null)
    if [ "$actual" = "$expected" ]; then
        echo "  PASS [$label]: file content matches"
        PASS=$((PASS + 1))
    else
        echo "  FAIL [$label]: expected '$expected', got '$actual'"
        FAIL=$((FAIL + 1))
    fi
}

assert_dir_contains_file() {
    local label="$1" dir="$2" filename="$3"
    if [ -f "$dir/$filename" ]; then
        echo "  PASS [$label]: $dir contains $filename"
        PASS=$((PASS + 1))
    else
        echo "  FAIL [$label]: $dir does not contain $filename"
        FAIL=$((FAIL + 1))
    fi
}

# Print summary and return non-zero if any test failed.
summarize() {
    local total=$((PASS + FAIL))
    echo ""
    echo "Results: $PASS/$total passed"
    [ "$FAIL" -eq 0 ]
}
