#!/usr/bin/python3

# Run the script from the root repo of Expander

import sys
import json
import subprocess

MPI_CONFIG = '''
{
    "n_groups": 2,
    "mpi_size_each_group": 8,
    "cpu_ids":
        [
            [0, 1, 2, 3, 4, 5, 6, 7],
            [8, 9, 10, 11, 12, 13, 14, 15]
        ]
}
'''

PROOF_CONFIG = '''
{
    "field": "fr",
    "circuit": "data/circuit_bn254.txt",
    "witness": "data/witness_bn254.txt",
    "gkr_proof": "data/gkr_proof.txt",
    "recursive_proof": "data/recursive_proof.txt"
}
'''

def parse_mpi_config(mpi_config):
    n_groups = mpi_config["n_groups"]
    mpi_size_each_group = mpi_config["mpi_size_each_group"]
    cpu_ids = mpi_config["cpu_ids"]

    if n_groups != len(cpu_ids):
        sys.exit("Lack/Too much cpu specifications.")

    # TODO: Check there are enough cpus on the machine
    for i in range(n_groups):
        if len(cpu_ids[i]) != mpi_size_each_group:
            sys.exit(f"Cpu ids are not correct for group {i}")

    return n_groups, mpi_size_each_group, cpu_ids

def parse_proof_config(proof_config):
    field = proof_config["field"]

    if field not in ["gf2ext128", "m31ext3", "fr"]:
        sys.exit("Unrecognized field, gkr now only supports gf2ext128, m31ext3 and fr")

    if field != "fr":
        sys.exit("Recursive proof only supports fr now")

    return proof_config["circuit"], proof_config["witness"], proof_config["gkr_proof"], proof_config["recursive_proof"]

DEBUG = True

# Run two mpi process
if __name__ == "__main__":
    mpi_config = json.loads(MPI_CONFIG)
    n_groups, mpi_size_each_group, cpu_ids = parse_mpi_config(mpi_config)

    proof_config = json.loads(PROOF_CONFIG)
    circuit, witness, gkr_proof, recursive_proof = parse_proof_config(proof_config)

    if DEBUG:
        n_groups = 1

    ps = []
    subprocess.run("RUSTFLAGS='-C target-feature=+avx512f' cargo build --release --bin expander-exec ", shell=True)
    for i in range(n_groups):
        cpu_id = ",".join(map(str, cpu_ids[i]))
        p = subprocess.Popen(["mpiexec", "-cpu-set", cpu_id, "-n", str(mpi_size_each_group), "./target/release/expander-exec", "prove", circuit, witness, gkr_proof + "." + str(i)])
        ps.append(p)

    for i in range(n_groups):
        ps[i].wait()

    print("gkr prove done.")

    for i in range(n_groups):
        subprocess.run(
            f'''
                cd recursion
                go run main.go -circuit={"../" + circuit} -witness={"../" + witness} -gkr_proof={"../" + gkr_proof + "." + str(i)} -recursive_proof={"../" + recursive_proof + "." + str(i)} -mpi_size={mpi_size_each_group}
                cd ..
            ''',
            shell=True,
        )