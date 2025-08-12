# Create data folder
mkdir data
cd data

# Download two repo for generating circuit and witness for GPU
git clone git@github.com:PolyhedraZK/ExpanderCompilerCollection.git
git clone git@github.com:PolyhedraZK/Expander.git

# Use Expander Compiler to generate Circuit and Witness for Expander
cd ExpanderCompilerCollection
cargo test --release keccak

# Move data to Expander
mkdir ../Expander/data
cp expander_compiler/*.txt ../Expander/data
cd ../Expander

# Use Expander's GPU serialization to produce circuit and witness for GPU usage
git checkout gpu-expander
EXPANDER_GPU=1 RUSTFLAGS="-C target-cpu=native -C target-feature=+avx512f" cargo run --release --bin=gkr -- --circuit keccak --pcs Raw --threads 1 --field m31ext3
EXPANDER_GPU=1 RUSTFLAGS="-C target-cpu=native -C target-feature=+avx512f" cargo run --release --bin=gkr -- --circuit keccak --pcs Raw --threads 1 --field goldilocks
EXPANDER_GPU=1 RUSTFLAGS="-C target-cpu=native -C target-feature=+avx512f" cargo run --release --bin=gkr -- --circuit keccak --pcs Raw --threads 1 --field fr
mv data/*.gpu.* ..
cd ..

# Remove this two repo
rm -rf ExpanderCompilerCollection
cd ..
