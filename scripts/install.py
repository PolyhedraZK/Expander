import subprocess
from sys import platform

if __name__ == "__main__":
    if platform == "darwin": # mac os
        subprocess.run(["brew", "install", "gcc", "make", "openmpi"])
    else:
        subprocess.run(["sudo", "apt-get", "update"])
        subprocess.run(["sudo", "apt-get", "install", "-y", "build-essential"])
