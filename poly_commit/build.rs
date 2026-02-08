// Build script for poly_commit crate.
// When the "cuda" feature is enabled, compiles the Orion CUDA kernels.

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

            build.include("cuda_orion");

            build
                .file("cuda_orion/orion_commit.cu")
                .compile("poly_commit_cuda_orion");

            println!("cargo:rustc-cfg=feature=\"cuda\"");
            println!("cargo:rerun-if-changed=cuda_orion");
        } else {
            println!("cargo:warning=nvcc not found, building poly_commit without CUDA support");
        }
    }
}
