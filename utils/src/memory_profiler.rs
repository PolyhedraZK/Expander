// Memory profiler for debugging memory usage
// Enable with feature "mem-profile"

#[cfg(feature = "mem-profile")]
use colored::Colorize;
#[cfg(feature = "mem-profile")]
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "mem-profile")]
pub static MEM_CALL_DEPTH: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "mem-profile")]
static BASELINE_MB: AtomicUsize = AtomicUsize::new(0);

/// Get current memory usage in MB (RSS - Resident Set Size)
#[cfg(feature = "mem-profile")]
pub fn get_memory_usage_mb() -> f64 {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<f64>() {
                            return kb / 1024.0;
                        }
                    }
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
        {
            if let Ok(rss_str) = String::from_utf8(output.stdout) {
                if let Ok(kb) = rss_str.trim().parse::<f64>() {
                    return kb / 1024.0;
                }
            }
        }
    }
    0.0
}

#[cfg(not(feature = "mem-profile"))]
pub fn get_memory_usage_mb() -> f64 {
    0.0
}

/// Set baseline memory for delta calculations
#[cfg(feature = "mem-profile")]
pub fn set_memory_baseline() {
    let current = get_memory_usage_mb() as usize;
    BASELINE_MB.store(current, Ordering::Relaxed);
}

#[cfg(not(feature = "mem-profile"))]
pub fn set_memory_baseline() {}

/// Get memory delta from baseline
#[cfg(feature = "mem-profile")]
pub fn get_memory_delta_mb() -> f64 {
    let current = get_memory_usage_mb();
    let baseline = BASELINE_MB.load(Ordering::Relaxed) as f64;
    current - baseline
}

#[cfg(not(feature = "mem-profile"))]
pub fn get_memory_delta_mb() -> f64 {
    0.0
}

/// Memory checkpoint - prints current memory usage with label
#[cfg(feature = "mem-profile")]
pub struct MemoryProfiler {
    label: String,
    start_memory: f64,
    enabled: bool,
}

#[cfg(feature = "mem-profile")]
impl MemoryProfiler {
    #[inline(always)]
    pub fn new(label: &str, enabled: bool) -> Self {
        let start_memory = get_memory_usage_mb();
        if enabled {
            MEM_CALL_DEPTH.fetch_add(1, Ordering::Relaxed);
            let depth = MEM_CALL_DEPTH.load(Ordering::Relaxed);
            let delta = get_memory_delta_mb();
            println!(
                "{:indent$}[MEM START] {} | RSS: {:.2} MB | Delta: {:+.2} MB",
                "",
                label.cyan().bold(),
                start_memory,
                delta,
                indent = 2 * depth
            );
        }
        Self {
            label: label.to_string(),
            start_memory,
            enabled,
        }
    }

    #[inline(always)]
    pub fn checkpoint(&self, checkpoint_label: &str) {
        if self.enabled {
            let current = get_memory_usage_mb();
            let delta_from_start = current - self.start_memory;
            let delta_from_baseline = get_memory_delta_mb();
            let depth = MEM_CALL_DEPTH.load(Ordering::Relaxed);
            println!(
                "{:indent$}  [MEM CHECKPOINT] {} :: {} | RSS: {:.2} MB | +{:.2} MB (this scope) | Delta: {:+.2} MB (total)",
                "",
                self.label.cyan(),
                checkpoint_label.yellow(),
                current,
                delta_from_start,
                delta_from_baseline,
                indent = 2 * depth
            );
        }
    }

    #[inline(always)]
    pub fn end(self) {
        if self.enabled {
            let end_memory = get_memory_usage_mb();
            let delta = end_memory - self.start_memory;
            let delta_from_baseline = get_memory_delta_mb();
            let depth = MEM_CALL_DEPTH.load(Ordering::Relaxed);
            let delta_str = if delta >= 0.0 {
                format!("+{:.2}", delta).red().to_string()
            } else {
                format!("{:.2}", delta).green().to_string()
            };
            println!(
                "{:indent$}[MEM END] {} | RSS: {:.2} MB | {} MB (this scope) | Delta: {:+.2} MB (total)",
                "",
                self.label.cyan().bold(),
                end_memory,
                delta_str,
                delta_from_baseline,
                indent = 2 * depth
            );
            MEM_CALL_DEPTH.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

#[cfg(not(feature = "mem-profile"))]
pub struct MemoryProfiler {}

#[cfg(not(feature = "mem-profile"))]
impl MemoryProfiler {
    #[inline(always)]
    pub fn new(_label: &str, _enabled: bool) -> Self {
        Self {}
    }

    #[inline(always)]
    pub fn checkpoint(&self, _checkpoint_label: &str) {}

    #[inline(always)]
    pub fn end(self) {}
}

/// Quick memory print macro
#[macro_export]
#[cfg(feature = "mem-profile")]
macro_rules! mem_print {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let mem = $crate::memory_profiler::get_memory_usage_mb();
        let delta = $crate::memory_profiler::get_memory_delta_mb();
        let depth = $crate::memory_profiler::MEM_CALL_DEPTH.load(core::sync::atomic::Ordering::Relaxed);
        println!(
            "{:indent$}[MEM] {} | RSS: {:.2} MB | Delta: {:+.2} MB",
            "",
            msg,
            mem,
            delta,
            indent = 2 * depth
        );
    }};
}

#[macro_export]
#[cfg(not(feature = "mem-profile"))]
macro_rules! mem_print {
    ($($arg:tt)*) => {{}};
}

/// Estimate size of a Vec in MB
#[inline(always)]
pub fn vec_size_mb<T>(v: &[T]) -> f64 {
    (v.len() * std::mem::size_of::<T>()) as f64 / (1024.0 * 1024.0)
}

/// Estimate size of nested Vec<Vec<T>> in MB
#[inline(always)]
pub fn nested_vec_size_mb<T>(v: &[Vec<T>]) -> f64 {
    v.iter().map(|inner| vec_size_mb(inner)).sum()
}
