#!/bin/bash

rm -rf tmp

mkdir tmp
cd tmp

# temp folder to store the circuit and witnesses
mkdir data

##########################
# ECC part
##########################
git clone https://github.com/PolyhedraZK/ExpanderCompilerCollection.git
cd ExpanderCompilerCollection 
# fixme: change to dev after https://github.com/PolyhedraZK/ExpanderCompilerCollection/pull/36 is merged
git switch zz/circuit_names

# generate keccak circuit and witnesses
cargo test --release keccak
cp expander_compiler/*.txt ../data

# generate poseidon circuit and witnesses
go run ecgo/examples/poseidon_m31/main.go
cp *.txt ../data

cd ../
ls -l data


##########################
# Expander part
##########################
git clone git@github.com:PolyhedraZK/Expander.git
cd Expander
# fixme: change to dev after https://github.com/PolyhedraZK/ExpanderCompilerCollection/pull/36 is merged
git switch zz/fix-poseidon-witnesses

mkdir data
mv ../data/*.txt data/
ls -l data

# run local tests
RUSTFLAGS="-C target-cpu=native -C target-feature=+avx512f" cargo +nightly run --release --bin=gkr -- -s keccak -f gf2ext128 -t 16
RUSTFLAGS="-C target-cpu=native -C target-feature=+avx512f" cargo +nightly run --release --bin=gkr -- -s keccak -f m31ext3 -t 16
RUSTFLAGS="-C target-cpu=native -C target-feature=+avx512f" cargo +nightly run --release --bin=gkr -- -s keccak -f fr -t 16
RUSTFLAGS="-C target-cpu=native -C target-feature=+avx512f" cargo +nightly run --release --bin=gkr -- -s poseidon -f m31ext3 -t 16

## SEEMS MPI will freeze?
# run mpi tests
RUSTFLAGS="-C target-cpu=native -C target-feature=+avx512f" cargo +nightly build --release --bin=gkr-mpi 

mpiexec -n 2 ./target/release/gkr-mpi -- -s keccak -f gf2ext128
mpiexec -n 2 ./target/release/gkr-mpi -- -s keccak -f m31ext3
mpiexec -n 2 ./target/release/gkr-mpi -- -s keccak -f fr
mpiexec -n 2 ./target/release/gkr-mpi -- -s poseidon -f m31ext3
