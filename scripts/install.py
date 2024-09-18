import subprocess
from sys import platform

if __name__ == "__main__":
    if platform == "darwin": # mac os
        subprocess.run(["brew", "install", "openmpi"])
    else:
        pass # Do nothing, assuming mpi has already been installed
