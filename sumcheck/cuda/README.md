# Sumcheck GPU Acceleration

This project implements GPU acceleration for the sumcheck protocol. The core computation leverages CUDA, and users can choose between CPU and GPU modes for computation. The field operations for BN254 and M31 extensions are supported, with `M31ext3` as the default field. 

## Installation

Make sure you have CUDA installed on your system.

### Compile the Project

To compile the project, simply run:

```bash
make clean && make
```

This will clean any existing binaries and generate a new one: `sumcheck.bin`.

## Usage

To run the program, use the following syntax:

```bash
./sumcheck.bin -m [cpu|gpu] -p [2^(size) of circuit] [-v]
```

For example, run 2^23 sumcheck on GPU, you can use

```bash
./sumcheck.bin -m gpu -p 23
```

### Options:
- `-m [cpu|gpu]`: Choose the computation mode. Default is `cpu`.
- `-p [circuit size]`: Specify the size of the circuit in powers of 2. Default is 20.
- `-v`: Enable verbose mode for detailed output.

## Field Support

The project supports different field operations based on compile-time flags:
- **BN254**: We use Ingonyama's Icicle as the underlying implementation for BN254 field operations.
- **M31ext3**: Default mode uses M31ext3 extension field.

To switch between fields, adjust the `USE_FIELD` variable in the `Makefile`. For example, to use BN254:

```bash
make clean && make USE_FIELD=useBN254
```

## Acknowledgments

We would like to express our sincere thanks to Ingonyama for providing the [Icicle framework](https://github.com/ingonyama-zk/icicle), which is used as the underlying implementation for BN254 field operations.
