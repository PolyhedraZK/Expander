import sys
import json
import subprocess

MPI_CONFIG_JSON = '''
{
    "field": "gf2ext128",
    "n_groups": 2,
    "mpi_size_each_group": 8,
    "cpu_ids":
        [
            [0, 1, 2, 3, 4, 5, 6, 7],
            [8, 9, 10, 11, 12, 13, 14, 15]
        ]
}
'''

def parse_config(mpi_config):
    field = mpi_config["field"]
    n_groups = mpi_config["n_groups"]
    mpi_size_each_group = mpi_config["mpi_size_each_group"]
    cpu_ids = mpi_config["cpu_ids"]
    
    if field not in ["gf2ext128", "m31ext3", "fr"]:
        sys.exit("Unrecognized field, now only supports gf2ext128, m31ext3 and fr")

    if n_groups != len(cpu_ids):
        sys.exit("Lack/Too much cpu specifications.")

    for i in range(n_groups):
        if len(cpu_ids[i]) != mpi_size_each_group:
            sys.exit(f"Cpu ids are not correct for group {i}")

    return field, n_groups, mpi_size_each_group, cpu_ids


# Run two mpi process
if __name__ == "__main__":
    mpi_config = json.loads(MPI_CONFIG_JSON)
    field, n_groups, mpi_size_each_group, cpu_ids = parse_config(mpi_config)

    for i in range(n_groups):
        cpu_id = ",".join(map(str, cpu_ids[i]))
        subprocess.Popen(["mpiexec", "-cpu-set", cpu_id, "-n", str(mpi_size_each_group), "./target/release/expander-rs-mpi", "-f", field])
