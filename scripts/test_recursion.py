#!/usr/bin/python3

# Run the script from the root repo of Expander

import os
import shutil
import subprocess

from dataclasses import dataclass
from enum import Enum
from typing import Any, Callable, Final


# TODO will need to pass it into recursion circuit for field info,
# now recursion circuit only supports bn254 gkr->groth16 recursion
class RecursiveProofField(Enum):
    GF2 = "gf2ext128"
    M31 = "m31ext3"
    FR = "fr"


@dataclass
class MPIConfig:
    cpu_ids: list[int]

    def __post_init__(self):
        mpi_group_size: int = len(self.cpu_ids)
        if mpi_group_size & (mpi_group_size - 1) != 0 or mpi_group_size == 0:
            raise Exception("mpi cpu group size should be a power of 2")

        if len(set(self.cpu_ids)) != len(self.cpu_ids):
            raise Exception("mpi cpu id contains duplications")

        # physical_cpus = psutil.cpu_count(logical=False)
        # if physical_cpus is None:
        #     raise Exception("hmmm your physical cpu count cannot be found")

        # sorted_cpu_ids = sorted(self.cpu_ids)
        # if sorted_cpu_ids[0] < 0 or sorted_cpu_ids[-1] >= physical_cpus:
        #     raise Exception(f"mpi cpu id should be in range [0, {physical_cpus}]")

    def cpus(self) -> int:
        return len(self.cpu_ids)

    def cpu_set_str(self) -> str:
        return ",".join([str(cpu_id) for cpu_id in self.cpu_ids])

    def mpi_prefix(self) -> str:
        return f"mpiexec -cpu-set {self.cpu_set_str()} -n {self.cpus()}"


MPI_CONFIG: Final[MPIConfig] = MPIConfig(
    cpu_ids=[0, 1]
)


@dataclass
class ProofConfig:
    field: RecursiveProofField
    circuit: str
    witness: str
    fs_hash_scheme: str
    pcs_scheme: str
    gkr_proof_prefix: str
    # recursive_proof: str


BN254_GKR_TO_GROTH16_RECURSION_PROOF_CONFIG: Final[ProofConfig] = ProofConfig(
    field=RecursiveProofField.FR,
    circuit="data/circuit_bn254.txt",
    witness="data/witness_bn254_mpi_2.txt",
    fs_hash_scheme="MIMC5",
    pcs_scheme="Raw",
    gkr_proof_prefix="data/bn254_gkr_proof.txt",
    # recursive_proof="data/recursive_proof.txt"
)


M31_GKR_TO_GKR_RECURSION_PROOF_CONFIG: Final[ProofConfig] = ProofConfig(
    field=RecursiveProofField.M31,
    circuit="data/small_circuit_m31.txt",
    witness="data/small_witness_m31.txt",
    fs_hash_scheme="Poseidon",
    pcs_scheme="Raw",
    gkr_proof_prefix="data/m31_gkr_proof.txt",
    # recursive_proof="data/recursive_proof.txt"
)


def change_working_dir():
    cwd = os.getcwd()
    if "Expander/scripts" in cwd:
        os.chdir("..")

def in_recursion_dir(closure: Callable[..., Any]):
    def wrapped():
        os.chdir("./recursion")
        closure()
        os.chdir("..")

    return wrapped


def gkr_proof_file(prefix: str, cpu_ids: list[int]) -> str:
    concatenated_cpu_ids = "-".join([str(i) for i in cpu_ids])
    return f"{prefix}.mpi-cpus-{concatenated_cpu_ids}"


def expander_compile():
    if subprocess.run("cargo build --release --bin expander-exec --features=recursion", shell=True).returncode != 0:
        raise Exception("build process is not returning 0")


def gkr_prove(proof_config: ProofConfig, mpi_config: MPIConfig) -> str:
    proof_file: str = gkr_proof_file(proof_config.gkr_proof_prefix, mpi_config.cpu_ids)
    prove_command_suffix: str = \
        f"./target/release/expander-exec \
        -f {proof_config.fs_hash_scheme} -p {proof_config.pcs_scheme} -c {proof_config.circuit} \
        prove -w {proof_config.witness} -o {proof_file}"

    prove_command: str = ' '.join(f"{mpi_config.mpi_prefix()} {prove_command_suffix}".split())
    print(prove_command)

    if subprocess.run(prove_command, shell=True).returncode != 0:
        raise Exception("prove process is not returning with 0")

    print("gkr prove done.")
    return proof_file

def vanilla_gkr_verify_check(
        proof_config: ProofConfig,
        proof_path: str,
        mpi_config: MPIConfig
):
    vanilla_verify_comand: str = \
        f"./target/release/expander-exec \
        -f {proof_config.fs_hash_scheme} -p {proof_config.pcs_scheme} -c {proof_config.circuit} \
        verify -w {proof_config.witness} -i {proof_path} -m {mpi_config.cpus()}"
    vanilla_verify_comand = ' '.join(vanilla_verify_comand.split())
    print(vanilla_verify_comand)

    if subprocess.run(vanilla_verify_comand, shell=True).returncode != 0:
        raise Exception("vanilla verify process is not returning with 0")

    print("gkr vanilla verify done.")


def test_bn254_gkr_to_groth16_recursion(
        proof_config: ProofConfig,
        mpi_config: MPIConfig
):
    proof_path = gkr_prove(proof_config, mpi_config)
    vanilla_gkr_verify_check(proof_config, proof_path, mpi_config)

    @in_recursion_dir
    def test_bn254_gkr_to_groth16_recursion_payload():
        bn254_gkr_to_gnark_cmd = ' '.join(f'''
        go run . groth16
        --circuit-file ../{proof_config.circuit}
        --witness-files ../{proof_config.witness}
        --gkr-proofs ../{proof_path}
        --mpi-size {mpi_config.cpus()}
        '''.strip().split())

        print(bn254_gkr_to_gnark_cmd)
        if subprocess.run(bn254_gkr_to_gnark_cmd, shell=True).returncode != 0:
            raise Exception("recursion proof is not proving correctly")

    test_bn254_gkr_to_groth16_recursion_payload()


def test_m31_gkr_to_gkr_recursion(
        proof_config: ProofConfig,
        mpi_config: MPIConfig
):
    shutil.copyfile("./scripts/small_circuit_m31.circ", "./data/small_circuit_m31.txt")
    shutil.copyfile("./scripts/small_witness_m31.circ", "./data/small_witness_m31.txt")

    proof_path = gkr_prove(proof_config, mpi_config)
    vanilla_gkr_verify_check(proof_config, proof_path, mpi_config)

    @in_recursion_dir
    def test_m31_gkr_to_gkr_recursion_payload():
        m31_gkr_cmd = ' '.join(f'''
        go run . mersenne31
        --circuit-file ../{proof_config.circuit}
        --witness-files ../{proof_config.witness}
        --gkr-proofs ../{proof_path}
        --mpi-size {mpi_config.cpus()}
        '''.strip().split())

        print(m31_gkr_cmd)
        if subprocess.run(m31_gkr_cmd, shell=True).returncode != 0:
            raise Exception("recursion proof is not proving correctly")

    test_m31_gkr_to_gkr_recursion_payload()


if __name__ == "__main__":
    # minor - check golang if exists on the machine
    if subprocess.run("go env", shell=True).returncode != 0:
        raise Exception("golang support missing")
    change_working_dir()
    expander_compile()

    # List of recursion test starts here
    test_bn254_gkr_to_groth16_recursion(
        BN254_GKR_TO_GROTH16_RECURSION_PROOF_CONFIG,
        MPI_CONFIG
    )

    # Note-ZF: Temporarily disabled m31 recursion test because the witness format has changed:
    #   The required number of witnesses should double the previous for mpi_size = 2
    #   I have no access to the witness generation source code. 
    # test_m31_gkr_to_gkr_recursion(
    #     M31_GKR_TO_GKR_RECURSION_PROOF_CONFIG,
    #     MPI_CONFIG
    # )
