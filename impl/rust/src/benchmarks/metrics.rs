//! System metrics collection for MITCH protocol benchmarking
//!
//! Provides CPU and memory usage monitoring during benchmark execution
//! for comprehensive performance analysis.

use std::time::Duration;
use sysinfo::{System, Pid};
use tokio::time::sleep;

/// Monitors CPU and memory usage during benchmarks.
#[derive(Debug)]
pub struct MetricsMonitor {
    system: System,
    pid: Pid,
    interval: Duration,
}

impl MetricsMonitor {
    pub fn new(interval: Duration) -> Self {
        let pid = Pid::from_u32(std::process::id());
        Self {
            system: System::new_all(),
            pid,
            interval,
        }
    }

    /// Starts monitoring and returns average CPU and memory usage.
    pub async fn start_monitoring(&mut self, duration: Duration) -> (f64, f64) {
        let mut cpu_samples = Vec::new();
        let mut mem_samples = Vec::new();
        let start_time = tokio::time::Instant::now();

        while start_time.elapsed() < duration {
            cpu_samples.push(self.current_cpu_usage());
            mem_samples.push(self.current_memory_mb());
            sleep(self.interval).await;
        }

        let avg_cpu = if cpu_samples.is_empty() { 0.0 } else { cpu_samples.iter().sum::<f64>() / cpu_samples.len() as f64 };
        let avg_mem = if mem_samples.is_empty() { 0.0 } else { mem_samples.iter().sum::<f64>() / mem_samples.len() as f64 };

        (avg_cpu, avg_mem)
    }

    /// Gets current CPU usage for the process.
    pub fn current_cpu_usage(&mut self) -> f64 {
        self.system.refresh_cpu();
        self.system.refresh_processes();
        if let Some(process) = self.system.process(self.pid) {
            process.cpu_usage() as f64
        } else {
            0.0
        }
    }

    /// Gets current memory usage in MB.
    pub fn current_memory_mb(&mut self) -> f64 {
        self.system.refresh_all();
        if let Some(process) = self.system.process(self.pid) {
            process.memory() as f64 / 1024.0 / 1024.0
        } else {
            0.0
        }
    }
}



