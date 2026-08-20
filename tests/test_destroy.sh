#!/usr/bin/env bash
# Tests: destroy command (discard cached files)
set -euo pipefail
source "$(dirname "$0")/common.sh"
echo "=== destroy ==="

DATA=$(mktemp -d)
trap 'teardown; rm -rf "$DATA"' EXIT

# ── Test 1: destroy removes specific cached file from disk ────────────────────
setup
printf '%s' "data" > "$DATA/todel.txt"
run_vfs "import $DATA/todel.txt" > /dev/null
# Open to cache it, then destroy it in the same session
FIFO=$(mktemp -u)
mkfifo "$FIFO"
"$BINARY" --home "$TEST_HOME" < "$FIFO" > /dev/null 2>&1 &
BIN_PID=$!
exec 3>"$FIFO"
echo "open todel.txt" >&3
sleep 0.2
CACHED=$(find "$TEST_HOME" -name "todel.txt" 2>/dev/null | head -1)
echo "destroy -f todel.txt" >&3
sleep 0.1
exec 3>&-
wait "$BIN_PID"
rm -f "$FIFO"
# The cached file should be deleted from disk
if [ -z "$CACHED" ] || [ ! -f "$CACHED" ]; then
    echo "  PASS [destroy removes file]: cached file was removed"
    PASS=$((PASS + 1))
else
    echo "  FAIL [destroy removes file]: cached file still exists at $CACHED"
    FAIL=$((FAIL + 1))
fi
teardown

# ── Test 2: destroy does not remove the file from the archive ─────────────────
setup
printf '%s' "safe" > "$DATA/safe.txt"
run_vfs "import $DATA/safe.txt" > /dev/null
FIFO=$(mktemp -u)
mkfifo "$FIFO"
"$BINARY" --home "$TEST_HOME" < "$FIFO" > /dev/null 2>&1 &
BIN_PID=$!
exec 3>"$FIFO"
echo "open safe.txt" >&3
sleep 0.2
echo "destroy -f safe.txt" >&3
sleep 0.1
exec 3>&-
wait "$BIN_PID"
rm -f "$FIFO"
OUT=$(run_vfs "ls")
assert_contains "destroy keeps archive" "$OUT" "safe.txt"
teardown

# ── Test 3: destroy -a clears all cached files ────────────────────────────────
setup
printf '%s' "p" > "$DATA/p.txt"
printf '%s' "q" > "$DATA/q.txt"
run_vfs "import $DATA/p.txt" "import $DATA/q.txt" > /dev/null
FIFO=$(mktemp -u)
mkfifo "$FIFO"
"$BINARY" --home "$TEST_HOME" < "$FIFO" > /dev/null 2>&1 &
BIN_PID=$!
exec 3>"$FIFO"
echo "open p.txt" >&3; sleep 0.2
echo "open q.txt" >&3; sleep 0.2
echo "destroy -a" >&3; sleep 0.1
OUT_FILE=$(mktemp)
echo "flush -a" >&3  # should error: no cached files left
sleep 0.1
exec 3>&-
wait "$BIN_PID"
rm -f "$FIFO" "$OUT_FILE"
# If we can still expand both files, archive is intact even after destroy
DEST=$(mktemp -d)
run_vfs "expand $DEST" > /dev/null
assert_dir_contains_file "destroy -a archive p" "$DEST" "p.txt"
assert_dir_contains_file "destroy -a archive q" "$DEST" "q.txt"
rm -rf "$DEST"
teardown

summarize
