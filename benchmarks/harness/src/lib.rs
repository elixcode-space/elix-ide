use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::thread;
use sysinfo::{System, Pid};

#[derive(Debug, Clone, serde::Serialize)]
pub struct BenchmarkResult {
    pub editor: String,
    pub test_name: String,
    pub iterations: usize,
    pub mean_ms: f64,
    pub median_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub std_dev_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MemorySnapshot {
    pub editor: String,
    pub rss_mb: f64,
    pub vms_mb: f64,
    pub cpu_percent: f64,
}

pub struct EditorBenchmark {
    name: String,
    command: String,
    args: Vec<String>,
    workspace: String,
}

impl EditorBenchmark {
    pub fn new(name: &str, command: &str, args: &[&str], workspace: &str) -> Self {
        Self {
            name: name.to_string(),
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            workspace: workspace.to_string(),
        }
    }

    pub fn measure_startup_time(&self, iterations: usize) -> BenchmarkResult {
        let mut times = Vec::with_capacity(iterations);
        
        for _ in 0..iterations {
            let start = Instant::now();
            let mut child = Command::new(&self.command)
                .args(&self.args)
                .arg("--wait")
                .arg(&self.workspace)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("Failed to start editor");
            
            child.wait().expect("Failed to wait for editor");
            let elapsed = start.elapsed();
            times.push(elapsed.as_millis() as f64);
            
            thread::sleep(Duration::from_millis(500));
        }
        
        self.calculate_stats(&times, "startup_time")
    }

    pub fn measure_file_open_latency(&self, files: &[String], iterations: usize) -> BenchmarkResult {
        let mut times = Vec::with_capacity(iterations * files.len());
        
        for _ in 0..iterations {
            for file in files {
                let start = Instant::now();
                let mut child = Command::new(&self.command)
                    .args(&self.args)
                    .arg("--wait")
                    .arg(file)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .expect("Failed to start editor");
                
                child.wait().expect("Failed to wait for editor");
                let elapsed = start.elapsed();
                times.push(elapsed.as_millis() as f64);
                
                thread::sleep(Duration::from_millis(200));
            }
        }
        
        self.calculate_stats(&times, "file_open_latency")
    }

    pub fn measure_memory_usage(&self, duration_secs: u64) -> MemorySnapshot {
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .arg(&self.workspace)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start editor");
        
        let pid = Pid::from_u32(child.id());
        thread::sleep(Duration::from_secs(3)); // Wait for startup
        
        let mut sys = System::new_all();
        let start = Instant::now();
        let mut max_rss = 0;
        let mut max_vms = 0;
        let mut total_cpu = 0.0;
        let mut samples = 0;
        
        while start.elapsed() < Duration::from_secs(duration_secs) {
            sys.refresh_processes();
            if let Some(process) = sys.process(pid) {
                let rss = process.memory();
                let vms = process.virtual_memory();
                let cpu = process.cpu_usage();
                
                max_rss = max_rss.max(rss);
                max_vms = max_vms.max(vms);
                total_cpu += cpu;
                samples += 1;
            }
            thread::sleep(Duration::from_millis(100));
        }
        
        child.kill().ok();
        
        MemorySnapshot {
            editor: self.name.clone(),
            rss_mb: max_rss as f64 / 1024.0,
            vms_mb: max_vms as f64 / 1024.0,
            cpu_percent: if samples > 0 { total_cpu / samples as f64 } else { 0.0 },
        }
    }

    pub fn measure_extension_host_latency(&self, iterations: usize) -> BenchmarkResult {
        // Measure time to activate extension host and run a simple command
        let mut times = Vec::with_capacity(iterations);
        
        for _ in 0..iterations {
            let start = Instant::now();
            // This would need editor-specific IPC - placeholder for now
            thread::sleep(Duration::from_millis(10));
            let elapsed = start.elapsed();
            times.push(elapsed.as_millis() as f64);
        }
        
        self.calculate_stats(&times, "extension_host_latency")
    }

    fn calculate_stats(&self, times: &[f64], test_name: &str) -> BenchmarkResult {
        let mut sorted = times.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let sum: f64 = times.iter().sum();
        let mean = sum / times.len() as f64;
        let median = sorted[sorted.len() / 2];
        let min = sorted[0];
        let max = sorted[sorted.len() - 1];
        
        let variance: f64 = times.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / times.len() as f64;
        let std_dev = variance.sqrt();
        
        let p95_idx = (times.len() as f64 * 0.95).ceil() as usize - 1;
        let p99_idx = (times.len() as f64 * 0.99).ceil() as usize - 1;
        let p95 = sorted[p95_idx.min(sorted.len() - 1)];
        let p99 = sorted[p99_idx.min(sorted.len() - 1)];
        
        BenchmarkResult {
            editor: self.name.clone(),
            test_name: test_name.to_string(),
            iterations: times.len(),
            mean_ms: mean,
            median_ms: median,
            min_ms: min,
            max_ms: max,
            std_dev_ms: std_dev,
            p95_ms: p95,
            p99_ms: p99,
        }
    }
}

pub fn run_comparison(workspace: &str) -> Vec<BenchmarkResult> {
    let editors = vec![
        EditorBenchmark::new("VSCode", "code", &[], workspace),
        EditorBenchmark::new("Cursor", "cursor", &[], workspace),
        EditorBenchmark::new("Elixir IDE", "cargo", &["run", "--release", "--manifest-path=src-tauri/Cargo.toml", "--"], workspace),
    ];
    
    let test_files: Vec<String> = (1..=20).map(|i| format!("{}/test_file_{}.rs", workspace, i)).collect();
    
    let mut results = Vec::new();
    
    for editor in &editors {
        println!("Benchmarking {}...", editor.name);
        
        results.push(editor.measure_startup_time(10));
        results.push(editor.measure_file_open_latency(&test_files, 5));
        results.push(editor.measure_extension_host_latency(20));
    }
    
    results
}