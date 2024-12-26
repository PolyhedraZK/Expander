use std::process::Command;
use std::env;

fn main() {
    // First check if mpicc is available
    let mpicc_check = Command::new("which")
        .arg("mpicc")
        .output();
    

    if let Err(_) = mpicc_check {
        println!("cargo:warning=mpicc not found, attempting to install...");
        
        // Detect the operating system
        let os = env::consts::OS;
        
        match os {
            "linux" => {
                // Try to detect the package manager
                let apt_check = Command::new("which")
                    .arg("apt")
                    .output();
                
                let dnf_check = Command::new("which")
                    .arg("dnf")
                    .output();
                
                if apt_check.is_ok() {
                    // Debian/Ubuntu
                    eprintln!("cargo:warning=Using apt to install OpenMPI...");
                    let status = Command::new("sudo")
                        .args(&["apt", "update"])
                        .status()
                        .expect("Failed to run apt update");
                    
                    if !status.success() {
                        panic!("Failed to update apt");
                    }
                    
                    let status = Command::new("sudo")
                        .args(&["apt", "install", "-y", "openmpi-bin", "libopenmpi-dev"])
                        .status()
                        .expect("Failed to install OpenMPI");
                    
                    if !status.success() {
                        panic!("Failed to install OpenMPI");
                    }
                } else if dnf_check.is_ok() {
                    // Fedora/RHEL
                    eprintln!("cargo:warning=Using dnf to install OpenMPI...");
                    let status = Command::new("sudo")
                        .args(&["dnf", "install", "-y", "openmpi", "openmpi-devel"])
                        .status()
                        .expect("Failed to install OpenMPI");
                    
                    if !status.success() {
                        panic!("Failed to install OpenMPI");
                    }
                } else {
                    panic!("Unsupported Linux distribution. Please install OpenMPI manually.");
                }
            },
            "macos" => {
                // Check for Homebrew
                let brew_check = Command::new("which")
                    .arg("brew")
                    .output();
                
                if brew_check.is_ok() {
                    eprintln!("cargo:warning=Using Homebrew to install OpenMPI...");
                    let status = Command::new("brew")
                        .args(&["install", "open-mpi"])
                        .status()
                        .expect("Failed to install OpenMPI");
                    
                    if !status.success() {
                        panic!("Failed to install OpenMPI");
                    }
                } else {
                    panic!("Homebrew not found. Please install Homebrew first or install OpenMPI manually.");
                }
            },
            _ => panic!("Unsupported operating system. Please install OpenMPI manually."),
        }
    }

    // After installation (or if already installed), set up compilation flags
    eprintln!("cargo:rustc-link-search=/usr/lib");
    eprintln!("cargo:rustc-link-lib=mpi");
    
    // Get MPI compilation flags
    let output = Command::new("mpicc")
        .arg("-show")
        .output()
        .expect("Failed to run mpicc");
    
    let flags = String::from_utf8_lossy(&output.stdout);
    
    // Parse the flags and add them to the build
    for flag in flags.split_whitespace() {
        if flag.starts_with("-L") {
            eprintln!("cargo:rustc-link-search=native={}", &flag[2..]);
        } else if flag.starts_with("-l") {
            eprintln!("cargo:rustc-link-lib={}", &flag[2..]);
        }
    }
    
    // Force rebuild if build.rs changes
    eprintln!("cargo:rerun-if-changed=build.rs");
}