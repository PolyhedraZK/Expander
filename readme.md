![Expander](https://github.com/PolyhedraZK/Expander/blob/master/data/logo.jpg)

# Expander-RS

Expander is a proof generation backend for Polyhedra Network. It aims to support fast proof generation.

This is the *rust version* of the "core" repo and more on "demo" purpose, we will continue develop on the repo to support more features.

For more technical introduction, visit our markdown files [here](https://github.com/PolyhedraZK/Expander/tree/master/docs/doc.md).

And [here](./tests/gkr_correctness.rs) for an example on how to use the gkr lib.

For more information, see the cpp version of the repo [here](https://github.com/PolyhedraZK/Expander).

## Environment Setup

Before executing setup, please make sure you read through the system requirements, and make sure your CPU is in the list.

```sh
wget -P data https://storage.googleapis.com/keccak8/circuit.txt
wget -P data https://storage.googleapis.com/keccak8/witness.txt
```


## Benchmarks

**Make sure you include `RUSTFLAGS="-C target-cpu=native"` to allow platform specific accelerations.**

Command template:

```sh
RUSTFLAGS="-C target-cpu=native" RUSTFLAGS="-C target-feature=+avx2" cargo run --release -- -f [fr|m31|m31ext3] -t [#threads] -s [keccak|poseidon]
```

Concretely if you are running on a 16 physical core CPU for Bn256 scalar field:

```sh
RUSTFLAGS="-C target-cpu=native" cargo run --release -- -f fr -t 16
```

## Correctness test

[Here](./tests/gkr_correctness.rs) we provide a test case for end-to-end proof generation and verification. 
To check the correctness, run the follow standard Rust test command:

```sh
RUSTFLAGS="-C target-cpu=native" cargo test --release -- --nocapture
```

## CLI

Usage:

```sh
RUSTFLAGS="-C target-cpu=native" cargo run --bin expander-exec --release -- prove <input:circuit_file> <input:witness_file> <output:proof>
RUSTFLAGS="-C target-cpu=native" cargo run --bin expander-exec --release -- verify <input:circuit_file> <input:witness_file> <input:proof>
RUSTFLAGS="-C target-cpu=native" cargo run --bin expander-exec --release -- serve <input:circuit_file> <input:ip> <input:port>
```

Example:

```sh
RUSTFLAGS="-C target-cpu=native" cargo run --bin expander-exec --release -- prove ./data/circuit.txt ./data/witness.txt ./data/out.bin
RUSTFLAGS="-C target-cpu=native" cargo run --bin expander-exec --release -- verify ./data/circuit.txt ./data/witness.txt ./data/out.bin
RUSTFLAGS="-C target-cpu=native" cargo run --bin expander-exec --release -- serve ./data/circuit.txt 127.0.0.1 3030
```

To test the service started by `expander-exec serve`, you can use the following command:
```sh
python ./scripts/test_http.py  # need "requests" package
```

## How to contribute?

Thank you for your interest in contributing to our project! We seek contributors with a robust background in cryptography and programming, aiming to improve and expand the capabilities of our proof generation system.

### Contribution Guidelines:

#### Pull Requests

We welcome your pull requests (PRs) and ask that you follow these guidelines to facilitate the review process:

- **General Procedure**:

  1. **Fork the repository** and clone it locally.
  2. **Create a branch** for your changes related to a specific issue or improvement.
  3. **Commit your changes**: Use clear and meaningful commit messages.
  4. **Push your changes** to your fork and then **submit a pull request** to the main repository.

- **PR Types and Specific Guidelines**:
  - **[BUG]** for bug fixes:
    - **Title**: Start with `[BUG]` followed by a brief description.
    - **Content**: Explain the issue being fixed, steps to reproduce, and the impact of the bug. Include any relevant error logs or screenshots.
    - **Tests**: Include tests that ensure the bug is fixed and will not recur.
  - **[FEATURE]** for new features:
    - **Title**: Start with `[FEATURE]` followed by a concise feature description.
    - **Content**: Discuss the benefits of the feature, possible use cases, and any changes it introduces to existing functionality.
    - **Documentation**: Update relevant documentation and examples.
    - **Tests**: Add tests that cover the new feature's functionality.
  - **[DOC]** for documentation improvements:
    - **Title**: Start with `[DOC]` and a short description of what is being improved.
    - **Content**: Detail the changes made and why they are necessary, focusing on clarity and accessibility.
  - **[TEST]** for adding or improving tests:
    - **Title**: Begin with `[TEST]` and describe the type of testing enhancement.
    - **Content**: Explain what the tests cover and how they improve the project's reliability.
  - **[PERF]** for performance improvements:
    - **Title**: Use `[PERF]` and a brief note on the enhancement.
    - **Content**: Provide a clear comparison of performance before and after your changes, including benchmarks or profiling data.
    - **Tests/Benchmarks**: Add tests that cover the new feature's functionality, and benchmarks to prove your improvement.

#### Review Process

Each pull request will undergo a review by one or more core contributors. We may ask for changes to better align with the project's goals and standards. Once approved, a maintainer will merge the PR.

We value your contributions greatly and are excited to see what you bring to this project. Let’s build something great together!
