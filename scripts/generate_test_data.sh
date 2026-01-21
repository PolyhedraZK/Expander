#!/bin/bash
# Generate test data (circuits, witnesses, and proofs) for CI
# This script generates all necessary test data and packages them for upload

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXPANDER_ROOT="$(dirname "$SCRIPT_DIR")"
WORK_DIR="$EXPANDER_ROOT/tmp_testdata"
OUTPUT_DIR="$EXPANDER_ROOT/testdata_output"

echo "=== Generating Expander Test Data ==="
echo "Work directory: $WORK_DIR"
echo "Output directory: $OUTPUT_DIR"

# Clean up previous runs
rm -rf "$WORK_DIR" "$OUTPUT_DIR"
mkdir -p "$WORK_DIR" "$OUTPUT_DIR"

cd "$WORK_DIR"

##########################
# Step 1: Generate circuits and witnesses using ExpanderCompilerCollection
##########################
echo ""
echo "=== Step 1: Cloning ExpanderCompilerCollection ==="
git clone --depth 1 -b dev https://github.com/PolyhedraZK/ExpanderCompilerCollection.git
cd ExpanderCompilerCollection

echo ""
echo "=== Step 2: Generating Keccak circuits and witnesses ==="
cargo test --release keccak 2>&1 | tail -20

# Copy generated files
cp expander_compiler/circuit_*.txt "$OUTPUT_DIR/" 2>/dev/null || true
cp expander_compiler/witness_*.txt "$OUTPUT_DIR/" 2>/dev/null || true

echo ""
echo "=== Step 3: Generating Poseidon circuits and witnesses ==="
go run ecgo/examples/poseidon_m31/main.go 2>&1 | tail -10
cp poseidon_*.txt "$OUTPUT_DIR/" 2>/dev/null || true

cd "$EXPANDER_ROOT"

echo ""
echo "=== Step 4: Listing generated circuit and witness files ==="
ls -la "$OUTPUT_DIR/"

##########################
# Step 2: Generate proofs using Expander
##########################
echo ""
echo "=== Step 5: Generating proofs ==="

# Create data directory and copy circuit/witness files
mkdir -p data
cp "$OUTPUT_DIR"/*.txt data/

# Build and run proof generation
cargo build --release --bin dev-setup

echo "Generating proofs for GF2, M31, and BN254..."
cargo run --release --bin dev-setup -- --compare 2>&1 | tail -30

# Copy generated proof files
cp data/proof_*.txt "$OUTPUT_DIR/" 2>/dev/null || true

echo ""
echo "=== Step 6: Final output files ==="
ls -la "$OUTPUT_DIR/"

##########################
# Step 3: Create archive for upload
##########################
echo ""
echo "=== Step 7: Creating archive ==="
cd "$OUTPUT_DIR"
tar -czvf ../expander-testdata.tar.gz *.txt

echo ""
echo "=== Done! ==="
echo "Archive created at: $EXPANDER_ROOT/expander-testdata.tar.gz"
echo ""
echo "Files included:"
tar -tzvf "$EXPANDER_ROOT/expander-testdata.tar.gz"

# Clean up work directory
rm -rf "$WORK_DIR"
