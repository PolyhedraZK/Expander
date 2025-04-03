#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print section headers
print_header() {
    echo -e "\n${YELLOW}================================${NC}"
    echo -e "${YELLOW}$1${NC}"
    echo -e "${YELLOW}================================${NC}\n"
}


RUSTFLAGS="-C target-feature=+avx512f" cargo run --release --bin gkr

# Function to run benchmark
run_benchmark() {
    local field=$1
    local pcs=$2
    local circuit=$3
    local threads=4
    local repeats=2

    print_header "Running benchmark: field=$field, pcs=$pcs, circuit=$circuit"
    
    ./target/release/gkr \
        --field "$field" \
        --pcs "$pcs" \
        --circuit "$circuit" \
        --threads "$threads" \
        --repeats "$repeats"
}

# Main benchmark execution
print_header "Starting GKR Benchmark Suite"

# M31 combinations
print_header "Testing M31 Field Combinations"
run_benchmark "m31ext3" "Raw" "keccak"
run_benchmark "m31ext3" "Raw" "poseidon"
run_benchmark "m31ext3" "Orion" "keccak"

# BN254 combinations
print_header "Testing BN254 Field Combinations"
run_benchmark "fr" "Raw" "keccak"
run_benchmark "fr" "Raw" "poseidon"
run_benchmark "fr" "Hyrax" "keccak"
run_benchmark "fr" "KZG" "poseidon"

# GF2 combinations
print_header "Testing GF2 Field Combinations"
run_benchmark "gf2ext128" "Raw" "keccak"
run_benchmark "gf2ext128" "Orion" "keccak"

# Goldilocks combinations
print_header "Testing Goldilocks Field Combinations"
run_benchmark "goldilocks" "Raw" "keccak"

print_header "Benchmark Suite Completed" 