#!/bin/bash
set -e

# Cleanup function
cleanup() {
    echo "Stopping all nodes..."
    pkill -f "cargo run --quiet --" || true
}
trap cleanup EXIT

# Clean old logs
rm -f node*.log

echo "Building..."
cargo build --quiet

# Start 5 Nodes (0, 1, 2, 3, 4)
echo "Starting 5 Nodes..."
RUST_LOG=info cargo run --quiet -- 0 > node0.log 2>&1 &
PID0=$!
sleep 2

RUST_LOG=info cargo run --quiet -- 1 > node1.log 2>&1 &
PID1=$!
RUST_LOG=info cargo run --quiet -- 2 > node2.log 2>&1 &
PID2=$!
RUST_LOG=info cargo run --quiet -- 3 > node3.log 2>&1 &
PID3=$!
RUST_LOG=info cargo run --quiet -- 4 > node4.log 2>&1 &
PID4=$!

echo "Nodes started. Waiting for View 1 and View 2 (approx 30s)..."
# View 1 and 2 should complete normally.

# Wait until we see activity for View 2?
# Let's just wait a fixed buffer. View 1 takes <5s (happy path). View 2 takes <5s.
sleep 15

# KILL NODE 3 (Leader for View 3)
echo "!!! KILLING NODE 3 (Leader View 3) !!!"
kill $PID3
echo "Node 3 killed. Waiting for timeout and recovery (View 3 -> Timeout -> View 4)..."

# View 3 is 30s timeout. So we wait >30s.
sleep 45

echo "--- LOG SUMMARY ---"
grep "FINALIZED BLOCK" node*.log | tail -n 5

echo ""
echo "--- CHECKING FOR DUMMY BLOCK / TIMEOUT QCs ---"
# We look for a QC where block_hash is all zeros (Dummy Hash)
grep "QC Formed" node*.log | grep "block_hash: 0000000000000000000000000000000000000000000000000000000000000000"

echo ""
echo "--- ALL RECEIVED BLOCKS (Node 0 - Last 3) ---"
grep "Received Block" node0.log | tail -n 3

echo ""
echo "--- CONSENSUS HEALTH CHECK ---"
# Verify we reached View 4 or 5
MAX_VIEW=$(grep "Received Block" node0.log | grep -o "view: [0-9]*" | cut -d " " -f 2 | sort -n | tail -1)
echo "Max View Reached: $MAX_VIEW"

if [ "$MAX_VIEW" -ge 4 ]; then
    echo "SUCCESS: Network recovered and advanced to View 4+."
else
    echo "FAILURE: Network stalled at View $MAX_VIEW."
    exit 1
fi
