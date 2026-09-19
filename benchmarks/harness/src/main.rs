use editor_bench::{run_comparison, BenchmarkResult, MemorySnapshot};
use std::fs;
use std::process::Command;

fn main() {
    // Create test workspace
    let workspace = format!("/tmp/editor_bench_{}", std::process::id());
    fs::create_dir_all(&workspace).unwrap();
    
    for i in 1..=20 {
        let content = format!(
            r#"fn main() {{
    println!("Test file {}");
    let data = vec![1, 2, 3, 4, 5];
    for item in data {{
        println!("{{}}", item * 2);
    }}
}}"#,
            i
        );
        fs::write(format!("{}/test_file_{}.rs", workspace, i), content).unwrap();
    }
    
    println!("Test workspace created at: {}", workspace);
    println!("Running benchmarks...\n");
    
    let results = run_comparison(&workspace);
    
    // Print results table
    println!("{:<15} {:<25} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}", 
        "Editor", "Test", "Mean", "Median", "Min", "Max", "StdDev", "P95", "P99");
    println!("{}", "-".repeat(120));
    
    for result in &results {
        println!("{:<15} {:<25} {:>10.2} {:>10.2} {:>10.2} {:>10.2} {:>10.2} {:>10.2} {:>10.2}",
            result.editor, result.test_name, result.mean_ms, result.median_ms,
            result.min_ms, result.max_ms, result.std_dev_ms, result.p95_ms, result.p99_ms);
    }
    
    // Memory benchmarks
    println!("\n\nMemory Usage Benchmarks:");
    println!("{:<15} {:>12} {:>12} {:>12}", "Editor", "RSS (MB)", "VMS (MB)", "CPU %");
    println!("{}", "-".repeat(55));
    
    for editor_name in &["VSCode", "Cursor", "Elixir IDE"] {
        let mem = measure_memory(editor_name, &workspace);
        println!("{:<15} {:>12.2} {:>12.2} {:>12.2}", 
            editor_name, mem.rss_mb, mem.vms_mb, mem.cpu_percent);
    }
    
    // Save JSON results
    let json = serde_json::to_string_pretty(&results).unwrap();
    fs::write("benchmark_results.json", json).unwrap();
    println!("\nResults saved to benchmark_results.json");
    
    // Cleanup
    fs::remove_dir_all(&workspace).ok();
}

fn measure_memory(editor: &str, workspace: &str) -> MemorySnapshot {
    let (cmd, args) = match editor {
        "VSCode" => ("code", vec![]),
        "Cursor" => ("cursor", vec![]),
        "Elixir IDE" => ("cargo", vec!["run", "--release", "--manifest-path=src-tauri/Cargo.toml", "--"]),
        _ => return MemorySnapshot { editor: editor.to_string(), rss_mb: 0.0, vms_mb: 0.0, cpu_percent: 0.0 },
    };
    
    let mut child = Command::new(cmd)
        .args(&args)
        .arg(workspace)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("Failed to start editor");
    
    let pid = sysinfo::Pid::from_u32(child.id());
    std::thread::sleep(std::time::Duration::from_secs(3));
    
    let mut sys = sysinfo::System::new_all();
    let start = std::time::Instant::now();
    let mut max_rss = 0;
    let mut max_vms = 0;
    let mut total_cpu = 0.0;
    let mut samples = 0;
    
    while start.elapsed() < std::time::Duration::from_secs(10) {
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
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    
    child.kill().ok();
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    MemorySnapshot {
        editor: editor.to_string(),
        rss_mb: max_rss as f64 / 1024.0,
        vms_mb: max_vms as f64 / 1024.0,
        cpu_percent: if samples > 0 { total_cpu / samples as f64 } else { 0.0 },
    }
}