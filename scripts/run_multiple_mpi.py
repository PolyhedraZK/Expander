import subprocess

# Run two mpi process
if __name__ == "__main__":
    subprocess.Popen(["mpiexec", "-n", "8", "./target/release/expander-rs", "-t", "1", "-f", "gf2ext128"])
    subprocess.Popen(["mpiexec", "-n", "8", "./target/release/expander-rs", "-t", "1", "-f", "gf2ext128"])
