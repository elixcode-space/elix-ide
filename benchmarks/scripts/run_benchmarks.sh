#!/usr/bin/env bash
# Benchmark runner for editor performance comparison
# Usage: ./run_benchmarks.sh [vscode|cursor|elixide|all]

set -e

EDITOR=${1:-all}
RESULTS_DIR="$(dirname "$0")/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT_FILE="$RESULTS_DIR/benchmark_report_$TIMESTAMP.md"

mkdir -p "$RESULTS_DIR"

echo "# Editor Performance Benchmark Report" > "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "Date: $(date)" >> "$REPORT_FILE"
echo "System: $(uname -a)" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

run_editor_benchmark() {
    local editor_name=$1
    local editor_cmd=$2
    local workspace=$3
    
    echo "## $editor_name" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    
    # Startup time
    echo "### Startup Time (cold)" >> "$REPORT_FILE"
    for i in {1..5}; do
        start=$(date +%s%3N)
        $editor_cmd --wait "$workspace" &
        pid=$!
        wait $pid
        end=$(date +%s%3N)
        duration=$((end - start))
        echo "- Run $i: ${duration}ms" >> "$REPORT_FILE"
    done
    echo "" >> "$REPORT_FILE"
    
    # Memory usage (RSS)
    echo "### Memory Usage (RSS)" >> "$REPORT_FILE"
    $editor_cmd "$workspace" &
    pid=$!
    sleep 5
    rss=$(ps -o rss= -p $pid 2>/dev/null || echo "N/A")
    echo "- RSS: ${rss} KB" >> "$REPORT_FILE"
    kill $pid 2>/dev/null || true
    sleep 2
    echo "" >> "$REPORT_FILE"
    
    # File open latency
    echo "### File Open Latency" >> "$REPORT_FILE"
    for i in {1..10}; do
        start=$(date +%s%3N)
        $editor_cmd --wait "$workspace/test_file_$i.rs" &
        pid=$!
        wait $pid
        end=$(date +%s%3N)
        duration=$((end - start))
        echo "- File $i: ${duration}ms" >> "$REPORT_FILE"
    done
    echo "" >> "$REPORT_FILE"
}

# Create test workspace
TEST_WORKSPACE="/tmp/editor_bench_$$"
mkdir -p "$TEST_WORKSPACE"
for i in {1..20}; do
    cat > "$TEST_WORKSPACE/test_file_$i.rs" <<EOF
fn main() {
    println!("Test file $i");
    let data = vec![1, 2, 3, 4, 5];
    for item in data {
        println!("{}", item * 2);
    }
}
EOF
done

case $EDITOR in
    vscode)
        run_editor_benchmark "VSCode" "code" "$TEST_WORKSPACE"
        ;;
    cursor)
        run_editor_benchmark "Cursor" "cursor" "$TEST_WORKSPACE"
        ;;
    elixide)
        run_editor_benchmark "ElixirIDE" "cargo run --release --manifest-path=$(pwd)/src-tauri/Cargo.toml --" "$TEST_WORKSPACE"
        ;;
    all)
        run_editor_benchmark "VSCode" "code" "$TEST_WORKSPACE"
        run_editor_benchmark "Cursor" "cursor" "$TEST_WORKSPACE"
        run_editor_benchmark "ElixirIDE" "cargo run --release --manifest-path=$(pwd)/src-tauri/Cargo.toml --" "$TEST_WORKSPACE"
        ;;
    *)
        echo "Unknown editor: $EDITOR"
        exit 1
        ;;
esac

# Cleanup
rm -rf "$TEST_WORKSPACE"

echo "## Summary" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "Results saved to: $REPORT_FILE" >> "$REPORT_FILE"
echo "Benchmark complete!" >> "$REPORT_FILE"

echo "Report generated: $REPORT_FILE"