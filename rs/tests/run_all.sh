#!/usr/bin/env bash
# Builds the binary then runs all bash integration tests.
# Usage: ./run_all.sh [--no-build]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BINARY="$SCRIPT_DIR/../target/debug/tag-vfs"

if [[ "${1:-}" != "--no-build" ]]; then
    echo "Building tag-vfs..."
    (cd "$SCRIPT_DIR/.." && cargo build 2>&1 | tail -3)
fi

if [ ! -x "$BINARY" ]; then
    echo "Binary not found at $BINARY. Run cargo build first."
    exit 1
fi

TOTAL_PASS=0
TOTAL_FAIL=0

run_suite() {
    local script="$1"
    local name
    name=$(basename "$script" .sh)

    # Each test script manages its own PASS/FAIL counters.
    # We capture the last "Results: X/Y" line.
    local output
    output=$(bash "$script" 2>&1)
    echo "$output"

    local result
    result=$(echo "$output" | grep "^Results:" | tail -1)
    local passed failed
    passed=$(echo "$result" | grep -oE '^Results: [0-9]+' | grep -oE '[0-9]+')
    local total_in
    total_in=$(echo "$result" | grep -oE '/[0-9]+ passed' | grep -oE '[0-9]+')

    passed="${passed:-0}"
    failed=$(( ${total_in:-0} - passed ))
    TOTAL_PASS=$(( TOTAL_PASS + passed ))
    TOTAL_FAIL=$(( TOTAL_FAIL + failed ))
}

TESTS=(
    test_import.sh
    test_remove.sh
    test_tag.sh
    test_ls.sh
    test_sz.sh
    test_flush.sh
    test_destroy.sh
    test_expand.sh
    test_reduce.sh
    test_merge.sh
    test_config.sh
    test_e2e.sh
)

for t in "${TESTS[@]}"; do
    echo ""
    echo "──────────────────────────────────────────────"
    run_suite "$SCRIPT_DIR/$t"
done

echo ""
echo "══════════════════════════════════════════════"
TOTAL=$(( TOTAL_PASS + TOTAL_FAIL ))
echo "TOTAL: $TOTAL_PASS / $TOTAL passed"
[ "$TOTAL_FAIL" -eq 0 ]
