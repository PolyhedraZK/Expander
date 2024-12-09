#!/usr/bin/python3

# Run the script from the root repo of Expander

import os
import sys
import json
import subprocess

MPI_CONFIG = '''
{
    "n_groups": 2,
    "mpi_size_each_group": 2,
    "cpu_ids":
        [
            [0, 1],
            [2, 3]
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

def change_working_dir():
    cwd = os.getcwd()
    if "Expander/scripts" in cwd:
        os.chdir("..")

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

    # FIXME(HS): working on M31 proof stuff now?
    if field != "fr":
        sys.exit("Recursive proof only supports fr now")

    return proof_config["circuit"], proof_config["witness"], proof_config["gkr_proof"], proof_config["recursive_proof"]


def gkr_proof_id(prefix: str, mpi_id: int) -> str:
    return f"{prefix}.mpi_id-{mpi_id}"


DEBUG = False

# Run two mpi process
if __name__ == "__main__":
    change_working_dir()

    mpi_config = json.loads(MPI_CONFIG)
    n_groups, mpi_size_each_group, cpu_ids = parse_mpi_config(mpi_config)

    proof_config = json.loads(PROOF_CONFIG)
    circuit, witness, gkr_proof, recursive_proof = parse_proof_config(proof_config)

    if DEBUG:
        n_groups = 1

    # NOTE(HS): as of 2024/12/09 - this command runs in CI environment, so mac naturally do not have
    # AVX 512 instructions - yet this is not quite a good condition statement, should be something like
    # archspec.  The work is deferred later as the current implementation suffices.
    avx_build_prefix: str = "" if sys.platform == 'darwin' else "RUSTFLAGS='-C target-feature=+avx512f'"
    compile_ret = subprocess.run(f"{avx_build_prefix} cargo build --release --bin expander-exec", shell=True)

    if compile_ret.returncode != 0:
        sys.exit(-1)

    # minor - check golang if exists on the machine
    if subprocess.run("go env", shell=True).returncode != 0:
        sys.exit(-1)

    # local function for mpi running prove process for each sub proof
    def mpi_prove(mpi_index: int) -> subprocess.Popen:
        mpi_cpus: str = ",".join([str(cpu_id) for cpu_id in cpu_ids[mpi_index]])
        mpi_command_prefix: str = f"mpiexec -cpu-set {mpi_cpus} -n {mpi_size_each_group}"

        literal_command = f"""
        {mpi_command_prefix}
        ./target/release/expander-exec prove {circuit} {witness} {gkr_proof_id(gkr_proof, mpi_index)}
        """.split()

        print(' '.join(literal_command))

        return subprocess.Popen(literal_command)

    ps: list[subprocess.Popen] = [mpi_prove(i) for i in range(n_groups)]
    ps_rets: list[int] = [p.wait() for p in ps]
    if not all([r == 0 for r in ps_rets]):
        sys.exit(-1)

    print("gkr prove done.")

    # FIXME(HS): construction field - working on compilation process,
    # need to work on MPI and recursive verifier on proof deserialization
    sys.exit(0)

    for i in range(n_groups):
        subprocess.run(
            f'''
                cd recursion
                go run main.go -circuit={"../" + circuit} -witness={"../" + witness} -gkr_proof={"../" + gkr_proof + "." + str(i)} -recursive_proof={"../" + recursive_proof + "." + str(i)} -mpi_size={mpi_size_each_group}
                cd ..
            ''',
            shell=True,
        )
