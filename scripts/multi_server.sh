#!/bin/bash

repeat_count=2

for ((i=0; i<repeat_count; i++))
do
    port=$((3030 + $i))
    echo "Running a circuit serve at $port"
    # RUSTFLAGS="-C target-cpu=native" cargo run --bin expander-exec --release -- serve  ../circuits/eth2/validator/gkr/circuit.txt 127.0.0.1 $port &
    RUSTFLAGS="-C target-cpu=native" cargo run --bin expander-exec --release -- serve  ../gnark-bls12_381/gkr/circuit_ate2.txt 127.0.0.1 $port &
    # RUSTFLAGS="-C target-cpu=native" cargo run --bin expander-exec --release -- serve  ../ExpanderCompilerCollection/examples/poseidon_m31/circuit.txt 127.0.0.1 $port &
    
    sleep 1
    echo "-------------------------"
done
