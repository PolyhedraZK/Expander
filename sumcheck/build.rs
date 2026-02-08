// Build script for sumcheck crate.
// When the "cuda" feature is enabled, compiles the CUDA kernels using nvcc.

fn main() {
    #[cfg(feature = "cuda")]
    {
        use std::env;

        // Detect nvcc
        let nvcc = match env::var("NVCC") {
            Ok(var) => which::which(var),
            Err(_) => which::which("nvcc"),
        };

        if let Ok(_nvcc_path) = nvcc {
            let mut build = cc::Build::new();
            build.cuda(true);
            // Target Ampere (sm_80) with backwards compat to Volta (sm_70)
            build.flag("-arch=sm_80");
            build.flag("-gencode").flag("arch=compute_70,code=sm_70");
            build.flag("-t0"); // Disable parallel compilation (avoids temp file conflicts)

            #[cfg(not(target_env = "msvc"))]
            {
                build.flag("-Xcompiler").flag("-Wno-unused-function");
            }

            // Include the cuda_m31 directory for headers
            build.include("cuda_m31");

            build
                .file("cuda_m31/m31_sumcheck.cu")
                .compile("sumcheck_cuda_m31");

            println!("cargo:rustc-cfg=feature=\"cuda\"");
            println!("cargo:rerun-if-changed=cuda_m31");
        } else {
            println!("cargo:warning=nvcc not found, building sumcheck without CUDA support");
        }
    }
}
