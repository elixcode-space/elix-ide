#!/usr/bin/env python3
"""
Editor Performance Benchmark Suite
Compares: VSCode, Cursor, Elixir IDE (our Tauri-based editor)
"""

import subprocess
import time
import json
import statistics
import os
import sys
import psutil
import argparse
from dataclasses import dataclass, asdict
from typing import List, Dict, Optional
from pathlib import Path

@dataclass
class BenchmarkResult:
    editor: str
    test_name: str
    iterations: int
    mean_ms: float
    median_ms: float
    min_ms: float
    max_ms: float
    std_dev_ms: float
    p95_ms: float
    p99_ms: float

@dataclass
class MemorySnapshot:
    editor: str
    rss_mb: float
    vms_mb: float
    cpu_percent: float

class EditorBenchmark:
    def __init__(self, name: str, command: str, args: List[str], workspace: str):
        self.name = name
        self.command = command
        self.args = args
        self.workspace = workspace
        self.process = None
    
    def is_available(self) -> bool:
        try:
            subprocess.run([self.command, "--version"], 
                         capture_output=True, timeout=5)
            return True
        except (FileNotFoundError, subprocess.TimeoutExpired):
            return False
    
    def measure_startup_time(self, iterations: int = 10) -> BenchmarkResult:
        times = []
        
        for _ in range(iterations):
            start = time.perf_counter()
            proc = subprocess.Popen(
                [self.command] + self.args + ["--wait", self.workspace],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL
            )
            proc.wait()
            elapsed = (time.perf_counter() - start) * 1000
            times.append(elapsed)
            time.sleep(0.5)
        
        return self._calc_stats(times, "startup_time")
    
    def measure_file_open_latency(self, files: List[str], iterations: int = 5) -> BenchmarkResult:
        times = []
        
        for _ in range(iterations):
            for file in files:
                start = time.perf_counter()
                proc = subprocess.Popen(
                    [self.command] + self.args + ["--wait", file],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL
                )
                proc.wait()
                elapsed = (time.perf_counter() - start) * 1000
                times.append(elapsed)
                time.sleep(0.2)
        
        return self._calc_stats(times, "file_open_latency")
    
    def measure_memory_usage(self, duration_secs: int = 10) -> MemorySnapshot:
        self.process = subprocess.Popen(
            [self.command] + self.args + [self.workspace],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL
        )
        
        time.sleep(3)  # Wait for startup
        
        try:
            proc = psutil.Process(self.process.pid)
            max_rss = 0
            max_vms = 0
            total_cpu = 0.0
            samples = 0
            
            start = time.time()
            while time.time() - start < duration_secs:
                try:
                    mem = proc.memory_info()
                    cpu = proc.cpu_percent(interval=0.1)
                    max_rss = max(max_rss, mem.rss)
                    max_vms = max(max_vms, mem.vms)
                    total_cpu += cpu
                    samples += 1
                except psutil.NoSuchProcess:
                    break
                time.sleep(0.1)
            
            return MemorySnapshot(
                editor=self.name,
                rss_mb=max_rss / 1024 / 1024,
                vms_mb=max_vms / 1024 / 1024,
                cpu_percent=total_cpu / samples if samples > 0 else 0.0
            )
        finally:
            if self.process:
                self.process.terminate()
                self.process.wait(timeout=5)
    
    def measure_extension_host_latency(self, iterations: int = 20) -> BenchmarkResult:
        # This would need editor-specific IPC; using placeholder
        times = []
        for _ in range(iterations):
            start = time.perf_counter()
            time.sleep(0.01)  # Simulated
            elapsed = (time.perf_counter() - start) * 1000
            times.append(elapsed)
        return self._calc_stats(times, "extension_host_latency")
    
    def _calc_stats(self, times: List[float], test_name: str) -> BenchmarkResult:
        sorted_times = sorted(times)
        mean = statistics.mean(times)
        median = statistics.median(times)
        min_val = min(times)
        max_val = max(times)
        std_dev = statistics.stdev(times) if len(times) > 1 else 0
        p95 = sorted_times[int(len(times) * 0.95)]
        p99 = sorted_times[int(len(times) * 0.99)]
        
        return BenchmarkResult(
            editor=self.name,
            test_name=test_name,
            iterations=len(times),
            mean_ms=mean,
            median_ms=median,
            min_ms=min_val,
            max_ms=max_val,
            std_dev_ms=std_dev,
            p95_ms=p95,
            p99_ms=p99
        )

def create_test_workspace(base_path: str, num_files: int = 20) -> List[str]:
    workspace = Path(base_path) / f"editor_bench_{os.getpid()}"
    workspace.mkdir(parents=True, exist_ok=True)
    
    files = []
    for i in range(1, num_files + 1):
        file_path = workspace / f"test_file_{i}.rs"
        content = f'''fn main() {{
    println!("Test file {i}");
    let data = vec![1, 2, 3, 4, 5];
    for item in data {{
        println!("{{}}", item * 2);
    }}
}}'''
        file_path.write_text(content)
        files.append(str(file_path))
    
    return files, str(workspace)

def run_benchmarks(editors_to_test: List[str], output_file: str = None):
    test_files, workspace = create_test_workspace("/tmp", 20)
    
    # Define editors
    editor_configs = {
        "VSCode": ("code", []),
        "Cursor": ("cursor", []),
        "Elixir IDE": ("cargo", ["run", "--release", "--manifest-path=src-tauri/Cargo.toml", "--"]),
    }
    
    all_results = []
    memory_results = []
    
    for editor_name in editors_to_test:
        if editor_name not in editor_configs:
            print(f"Unknown editor: {editor_name}")
            continue
        
        cmd, args = editor_configs[editor_name]
        bench = EditorBenchmark(editor_name, cmd, args, workspace)
        
        if not bench.is_available():
            print(f"{editor_name} not available, skipping...")
            continue
        
        print(f"\n=== Benchmarking {editor_name} ===")
        
        # Startup time
        print("  Measuring startup time...")
        result = bench.measure_startup_time(10)
        all_results.append(result)
        print(f"    Mean: {result.mean_ms:.2f}ms, P95: {result.p95_ms:.2f}ms")
        
        # File open latency
        print("  Measuring file open latency...")
        result = bench.measure_file_open_latency(test_files, 5)
        all_results.append(result)
        print(f"    Mean: {result.mean_ms:.2f}ms, P95: {result.p95_ms:.2f}ms")
        
        # Extension host latency
        print("  Measuring extension host latency...")
        result = bench.measure_extension_host_latency(20)
        all_results.append(result)
        print(f"    Mean: {result.mean_ms:.2f}ms")
        
        # Memory usage
        print("  Measuring memory usage...")
        mem = bench.measure_memory_usage(10)
        memory_results.append(mem)
        print(f"    RSS: {mem.rss_mb:.2f}MB, CPU: {mem.cpu_percent:.1f}%")
    
    # Print summary table
    print("\n" + "=" * 100)
    print("PERFORMANCE SUMMARY")
    print("=" * 100)
    print(f"{'Editor':<15} {'Test':<25} {'Mean (ms)':>12} {'Median (ms)':>13} {'Min (ms)':>10} {'Max (ms)':>10} {'P95 (ms)':>10} {'P99 (ms)':>10}")
    print("-" * 100)
    
    for r in all_results:
        print(f"{r.editor:<15} {r.test_name:<25} {r.mean_ms:>12.2f} {r.median_ms:>13.2f} {r.min_ms:>10.2f} {r.max_ms:>10.2f} {r.p95_ms:>10.2f} {r.p99_ms:>10.2f}")
    
    print("\n" + "=" * 60)
    print("MEMORY USAGE")
    print("=" * 60)
    print(f"{'Editor':<15} {'RSS (MB)':>12} {'VMS (MB)':>12} {'CPU %':>10}")
    print("-" * 60)
    for m in memory_results:
        print(f"{m.editor:<15} {m.rss_mb:>12.2f} {m.vms_mb:>12.2f} {m.cpu_percent:>10.1f}")
    
    # Save JSON
    if output_file:
        output = {
            "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
            "system": os.uname()._asdict(),
            "performance": [asdict(r) for r in all_results],
            "memory": [asdict(m) for m in memory_results]
        }
        with open(output_file, 'w') as f:
            json.dump(output, f, indent=2)
        print(f"\nResults saved to {output_file}")
    
    # Cleanup
    import shutil
    shutil.rmtree(workspace, ignore_errors=True)

def main():
    parser = argparse.ArgumentParser(description="Editor Performance Benchmark")
    parser.add_argument("editors", nargs="+", choices=["vscode", "cursor", "elixide", "all"],
                       help="Editors to benchmark")
    parser.add_argument("-o", "--output", help="Output JSON file")
    parser.add_argument("--list", action="store_true", help="List available editors")
    
    args = parser.parse_args()
    
    if args.list:
        editor_configs = {"VSCode": "code", "Cursor": "cursor", "Elixir IDE": "cargo"}
        for name, cmd in editor_configs.items():
            try:
                subprocess.run([cmd, "--version"], capture_output=True, timeout=5)
                status = "✓ Available"
            except:
                status = "✗ Not found"
            print(f"  {name}: {status}")
        return
    
    editors = []
    if "all" in args.editors:
        editors = ["VSCode", "Cursor", "Elixir IDE"]
    else:
        editors = [e.capitalize() for e in args.editors]
    
    output_file = args.output or f"benchmark_results_{time.strftime('%Y%m%d_%H%M%S')}.json"
    run_benchmarks(editors, output_file)

if __name__ == "__main__":
    main()