<div align="center" style="width: 100%;">
  <img 
    src="https://expander.polyhedra.network/assets/static/logo-with-text.16d5af29.svg" 
    alt="Expander Logo"
    style="width: 400px; height: auto;"
  />
</div>


# Expander GPU Acceleration

Expander is a proof generation backend for Polyhedra Network. It aims to support fast proof generation.

Expander now includes a high-performance GPU backend powered by CUDA, designed to dramatically accelerate proof generation. This backend is optimized for NVIDIA GPUs and offers significant speedups, especially for complex circuits and large-scale computations.

### Key Features

- **Massive Parallelism**: Leverages the full power of modern GPUs to process thousands of proofs in parallel.
- **MPI Merge**: Introduces an innovative "MPI Merge" feature that can compress proofs from thousands of independent computations into a single, compact proof. In our tests, we've achieved a compression ratio of up to `16384:1`. This is particularly useful in scenarios with large batches of similar computations.
- **Broad Field Support**: The GPU backend supports multiple field types, including `BN254`, `Goldilocks`, and `M31`.

### System Requirements

- **NVIDIA GPU**: A CUDA-enabled NVIDIA GPU with compute capability 7.0+ is recommended.
- **CUDA Toolkit**: Version 12.5 or newer.
- **Compiler**: `clang` and `clang++`.
- **Build Tools**: `cmake` (version 3.18+) and `ninja`.

### Build Instructions

The current release of Expander-GPU is in binary form. Please contact us if you are interested in source code access.

## GPU Benchmarks

The GPU backend delivers substantial performance improvements over the CPU implementation. The following benchmarks were run on an NVIDIA GPU, showcasing the throughput for various configurations.

### Performance Results

| Field            | Throughput (8192 proofs) | Throughput (16384 proofs, MPI Merged) |
|------------------|--------------------------|---------------------------------------|
| `m31ext3`        | ~2788 proofs/sec         | ~3040 computations/sec                |
| `goldilocksext2` | ~2597 proofs/sec         | ~2255 computations/sec                |
| `bn254`          | ~1313 proofs/sec         | ~1525 computations/sec                |

**Note on BN254 Performance**: The GPU acceleration is particularly impactful for the `BN254` field. Compared to our highly optimized AVX512 CPU backend, **the GPU implementation provides a 7-10x speedup** compared to AMD 9950X3D, achieving over 1500 merged computations per second. This makes Expander an ideal choice for ZK applications built on Ethereum-friendly curves.

### Running Benchmarks Manually and Profiling

You can reproduce these benchmarks using the `Makefile`:

```sh
# Run standard benchmark with 8192 parallel proofs
make profile

# Run benchmark with 16384 parallel proofs and MPI merge enabled
make mpi-profile

# Run standard benchmark with detailed profiling data
make profile PROFILE_LEVEL=2

# Run standard benchmark with detailed profiling data
make mpi-profile PROFILE_LEVEL=2
```

You can customize the `FIELD_TYPE` and `PROFILE_LEVEL` variables in the `Makefile` to test different configurations. You should be able to see a detailed profiling report as below.

```
====== GKR System Initialization ======
Parsed RZ0 Challenge: 0x128e207ced0a98b1401e2e521465544111847e131de192a5f527ecbd1611d6b0

GPU Memory Allocation Summary:
  Circuit:      29.47 MB (30898000 B)
  Transcript:   4.03 GB (4330817408 B)
  Scratchpad:   14.25 GB (15303180288 B)
  Total:        18.31 GB (19664895696 B)

MPI Merge Status:
  MPI Length:           8192 (independent computations)
  Number of Proofs:     8192 (final transcripts)
  MPI Merge Enabled:    NO

System Configuration:
  Circuit Layers:       144 layers
  MPI Length:           8192
  Enable MPI Merge:     false
  Field Type:           bn254
  Fiat-Shamir Type:     sha2-256
  Max Input Variables:  13
  Max Output Variables: 13

Prove Done! Final Claims:
  vx_claim = [0x08d2107f3419f056dda4310fd9de72a8eca95840b26a20068d70262ea9495086]
  vy_claim = [0x18e99e28f39df8da3cea05e6382991fa69fb57053d500112de6dab091267656c]

====== GKR Hierarchical Profiling Results (with GPU timing) ======
Function Name                            Call Count   Total Time (s)  Avg Time (ms)   % of Total
---------------------------------------- ------------ --------------- --------------- ----------
Sumcheck                                 287          2.790811        9.724           49.89    %
  - receive_challange                    3513         1.292797        0.368           23.11    %
  - poly_eval_at                         3513         1.140716        0.325           20.39    %
  - Fiat-shamir(sumcheck)                3513         0.329160        0.094           5.88     %
  - Apply phase 2 coef                   1754         0.021809        0.012           0.39     %
Prepare H(x)                             144          1.603928        11.138          28.67    %
  - eq_eval_at                           287          0.682776        2.379           12.21    %
    - eq_eval_combine                    287          0.388053        1.352           6.94     %
    - scatter_to_build_eq_buf            3504         0.241123        0.069           4.31     %
    - scatter_to_first_element           574          0.034887        0.061           0.62     %
  - build_hgx_mult_and_add               144          0.616053        4.278           11.01    %
    - build_hgx_mult                     143          0.376732        2.634           6.74     %
    - build_hgx_add                      144          0.237986        1.653           4.25     %
  - acc_from_rx_to_rz0                   142          0.379750        2.674           6.79     %
  - memset_clear_x_vals                  143          0.275147        1.924           4.92     %
Prepare H(y)                             143          1.182382        8.268           21.14    %
  - build_hgy_mult_only                  143          0.463457        3.241           8.29     %
  - memset_clear_y_vals                  143          0.367486        2.570           6.57     %
Fiat-shamir(gkr)                         717          0.016354        0.023           0.29     %
TOTAL                                    -            5.593475        -               100.00%   
=============================================

====== Expander-GPU Performance Metrics ======
Field element type:   bn254
Fiat-shamir type:     sha2-256
GKR proof size:       379232 bytes
GKR proof time:       5.594314 seconds
Proofs per second:    1464.34 proof/sec
```

## Acknowledgments

The code of Expander-GPU is derived from the [ICICLE project](https://github.com/ingonyama-zk/icicle). 
We are grateful to the ICICLE team for their contributions to the community, providing efficient field element operations on GPU that enable high-performance cryptographic computations.
