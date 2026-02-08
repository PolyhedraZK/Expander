// Build script for tree crate.
// When the "cuda" feature is enabled, compiles Keccak-256 Merkle tree CUDA kernels.

fn main() {
    #[cfg(feature = "cuda")]
    {
        use std::env;

        let nvcc = match env::var("NVCC") {
            Ok(var) => which::which(var),
            Err(_) => which::which("nvcc"),
        };

        if let Ok(_nvcc_path) = nvcc {
            let mut build = cc::Build::new();
            build.cuda(true);
            build.flag("-arch=sm_80");
            build.flag("-gencode").flag("arch=compute_70,code=sm_70");
            build.flag("-t0");

            #[cfg(not(target_env = "msvc"))]
            {
                build.flag("-Xcompiler").flag("-Wno-unused-function");
            }

            build.include("cuda");

            build
                .file("cuda/keccak256_merkle.cu")
                .compile("tree_cuda_keccak");

            println!("cargo:rustc-cfg=feature=\"cuda\"");
            println!("cargo:rerun-if-changed=cuda");
        } else {
            println!("cargo:warning=nvcc not found, building tree without CUDA support");
        }
    }
}
