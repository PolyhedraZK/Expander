fn main() {
    // Only build CUDA PCS kernel when cuda_pcs feature is enabled AND nvcc is available
    #[cfg(feature = "cuda_pcs")]
    {
        if std::process::Command::new("nvcc").arg("--version").output().is_ok() {
            cc::Build::new()
                .cuda(true)
                .flag("-O3")
                .flag("-std=c++17")
                .file("cuda/pcs_linear_combine.cu")
                .file("cuda/gpu_commit.cu")
                .compile("pcs_cuda");
            println!("cargo:rerun-if-changed=cuda/gpu_commit.cu");
            println!("cargo:rustc-link-lib=cudart");
            println!("cargo:rerun-if-changed=cuda/pcs_linear_combine.cu");
        } else {
            eprintln!("Warning: nvcc not found, cuda_pcs feature disabled at build time");
        }
    }
}
