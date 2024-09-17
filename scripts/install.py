import subprocess
from sys import platform

if __name__ == "__main__":
    if platform == "darwin":
        subprocess.run(["brew", "install", "openmpi"])
    else:
        subprocess.run(["wget", "https://download.open-mpi.org/release/open-mpi/v5.0/openmpi-5.0.5.tar.gz"])
        subprocess.run(["tar", "xf", "openmpi-5.0.5.tar.bz2"])
        subprocess.run(["cd", "openmpi-5.0.5"], shell=True)
        subprocess.run(["./configure", "--prefix=/tmp"])
        subprocess.run(["make", "-j", "all"])
        subprocess.run(["make", "install"])
        subprocess.run(["cd", ".."], shell=True)
