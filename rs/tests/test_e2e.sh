#!/usr/bin/env bash
# End-to-end integration test: realistic multi-step workflow.
# Sequence: import → tag → ls/sz → open/modify/flush → remove → expand
#           → reduce → merge (with tags) → config
set -euo pipefail
source "$(dirname "$0")/common.sh"
echo "=== end-to-end ==="

DATA=$(mktemp -d)
trap 'teardown; rm -rf "$DATA"' EXIT
setup

# ── Create test data ──────────────────────────────────────────────────────────
printf '%s' "quarterly numbers"   > "$DATA/report.txt"       # 17 bytes
printf '%s' "meeting notes here"  > "$DATA/notes.txt"        # 17 bytes
printf '%s' "main() { return 0; }" > "$DATA/main.c"         # 18 bytes
printf '%s' "readme text"         > "$DATA/readme.txt"       # 11 bytes

# ── Phase 1: Import files ─────────────────────────────────────────────────────
echo "--- Phase 1: import"
run_vfs "import $DATA/report.txt" \
        "import $DATA/notes.txt"  \
        "import $DATA/main.c"     \
        "import $DATA/readme.txt" > /dev/null

OUT=$(run_vfs "ls")
assert_contains "p1 report"  "$OUT" "report.txt"
assert_contains "p1 notes"   "$OUT" "notes.txt"
assert_contains "p1 main"    "$OUT" "main.c"
assert_contains "p1 readme"  "$OUT" "readme.txt"

# ── Phase 2: Tag files ────────────────────────────────────────────────────────
echo "--- Phase 2: tag"
run_vfs "tag -f report.txt -t work"                > /dev/null
run_vfs "tag -f notes.txt  -t work"                > /dev/null
run_vfs "tag -f main.c     -t code"                > /dev/null
run_vfs "tag -f report.txt -t finance"             > /dev/null

OUT_WORK=$(run_vfs "ls work")
OUT_CODE=$(run_vfs "ls code")
assert_contains     "p2 work report" "$OUT_WORK" "report.txt"
assert_contains     "p2 work notes"  "$OUT_WORK" "notes.txt"
assert_not_contains "p2 work no c"   "$OUT_WORK" "main.c"
assert_contains     "p2 code main"   "$OUT_CODE" "main.c"
assert_not_contains "p2 code no rep" "$OUT_CODE" "report.txt"

# ── Phase 3: Size checks ──────────────────────────────────────────────────────
echo "--- Phase 3: sz"
OUT_SZ_ALL=$(run_vfs "sz")
OUT_SZ_WORK=$(run_vfs "sz work")

# Total size = 17 + 17 + 18 + 11 = 63 bytes
assert_contains "p3 total sz" "$OUT_SZ_ALL" "63.0 B"
# Work tag size = 17 (report) + 17 (notes) = 34 bytes
assert_contains "p3 work sz"  "$OUT_SZ_WORK" "34.0 B"

# ── Phase 4: Open, modify, and flush a file ───────────────────────────────────
echo "--- Phase 4: flush with modification"
FIFO=$(mktemp -u)
mkfifo "$FIFO"
"$BINARY" --home "$TEST_HOME" < "$FIFO" > /dev/null 2>&1 &
BIN_PID=$!
exec 3>"$FIFO"
echo "open report.txt" >&3
sleep 0.3
CACHED=$(find "$TEST_HOME" -name "report.txt" 2>/dev/null | head -1)
if [ -n "$CACHED" ]; then
    printf '%s' "updated quarterly numbers" > "$CACHED"
fi
echo "flush -f report.txt" >&3
exec 3>&-
wait "$BIN_PID"
rm -f "$FIFO"

DEST_P4=$(mktemp -d)
run_vfs "expand $DEST_P4" > /dev/null
assert_file_content "p4 flush content" "$DEST_P4/report.txt" "updated quarterly numbers"
rm -rf "$DEST_P4"

# ── Phase 5: Remove a file ────────────────────────────────────────────────────
echo "--- Phase 5: remove"
run_vfs "remove -f readme.txt" > /dev/null
OUT=$(run_vfs "ls")
assert_not_contains "p5 removed"    "$OUT" "readme.txt"
assert_contains     "p5 others ok"  "$OUT" "report.txt"

# ── Phase 6: Expand to verify archive state ───────────────────────────────────
echo "--- Phase 6: expand"
DEST_P6=$(mktemp -d)
run_vfs "expand $DEST_P6" > /dev/null
assert_dir_contains_file "p6 report"  "$DEST_P6" "report.txt"
assert_dir_contains_file "p6 notes"   "$DEST_P6" "notes.txt"
assert_dir_contains_file "p6 main"    "$DEST_P6" "main.c"
if [ -f "$DEST_P6/readme.txt" ]; then
    echo "  FAIL [p6 readme removed]: readme.txt should not exist after remove"
    FAIL=$((FAIL + 1))
else
    echo "  PASS [p6 readme removed]: readme.txt correctly absent"
    PASS=$((PASS + 1))
fi
assert_file_content "p6 flushed report" "$DEST_P6/report.txt" "updated quarterly numbers"
rm -rf "$DEST_P6"

# ── Phase 7: Reduce (compress new files into the archive) ────────────────────
echo "--- Phase 7: reduce"
printf '%s' "patch notes" > "$DATA/changelog.txt"
run_vfs "reduce $DATA/changelog.txt" > /dev/null
OUT=$(run_vfs "ls")
assert_contains "p7 changelog" "$OUT" "changelog.txt"

# ── Phase 8: Merge a second archive ──────────────────────────────────────────
echo "--- Phase 8: merge"
ARCHIVE_B=$(mktemp -d)
printf '%s' "design spec" > "$DATA/spec.txt"
printf '%s' "unit tests"  > "$DATA/tests.txt"
printf "import $DATA/spec.txt\nimport $DATA/tests.txt\ntag -f spec.txt -t design\n" | \
    "$BINARY" --home "$ARCHIVE_B" > /dev/null 2>&1

run_vfs "merge $ARCHIVE_B/archive.dat" > /dev/null
OUT=$(run_vfs "ls")
assert_contains "p8 spec"  "$OUT" "spec.txt"
assert_contains "p8 tests" "$OUT" "tests.txt"
# Verify merged tag survived
OUT_DESIGN=$(run_vfs "ls design")
assert_contains "p8 design tag" "$OUT_DESIGN" "spec.txt"
rm -rf "$ARCHIVE_B"

# ── Phase 9: Config change ────────────────────────────────────────────────────
echo "--- Phase 9: config"
OUT=$(run_vfs "config cliPrefix [vfs]" "ls")
assert_contains "p9 new prefix" "$OUT" "[vfs]"

teardown
summarize
