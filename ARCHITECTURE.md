# ElixirIDE Architecture Blueprint

## System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           ELIXIDE ARCHITECTURE                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐     │
│  │   FRONTEND       │    │   BACKEND        │    │   DEPLOYMENT     │     │
│  │   (React/TS)     │◄──►│   (Rust/Tauri)   │◄──►│   (Nanos Unik.)  │     │
│  └──────────────────┘    └──────────────────┘    └──────────────────┘     │
│         │                       │                       │                  │
│         ▼                       ▼                       ▼                  │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐     │
│  │ CoreEngineService│    │ core_engine.rs   │    │ HTTP/WS Server   │     │
│  │ PiscisToolsService│   │ piscis_tools.rs  │    │ (Axum)           │     │
│  │ Workbench        │    │ svm_service.rs   │    │ 8080 port        │     │
│  └──────────────────┘    └──────────────────┘    └──────────────────┘     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Dual Deployment Modes

### Mode 1: Local Development (Tauri Desktop)
```
┌─────────────────────────────────────────────────────────────────┐
│                        TAURI APP                                 │
├─────────────────────────────────────────────────────────────────┤
│  Frontend (WebView)                                             │
│  ├── CoreEngineService  ──► invoke() ──► initialize_core()     │
│  ├── PiscisToolsService ──► invoke() ──► execute_piscis_tool() │
│  └── Workbench          ──► listen() ──► Tauri Events          │
├─────────────────────────────────────────────────────────────────┤
│  Backend (Rust)                                                 │
│  ├── core_engine.rs          ▸ piscis_kernel::headless         │
│  │   ├── KernelState (DB + Settings)                            │
│  │   ├── ToolRegistry (13 tools)                                │
│  │   └── EventSink (Tauri Window emitter)                      │
│  ├── piscis_tools.rs        ▸ ToolRegistry::get().call()       │
│  ├── svm_service.rs         ▸ libsvm_rs::predict               │
│  └── Commands (17 total)                                        │
└─────────────────────────────────────────────────────────────────┘
```

### Mode 2: Production/Edge (Nanos Unikernel)
```
┌─────────────────────────────────────────────────────────────────┐
│                      NANOS UNIKERNEL                             │
├─────────────────────────────────────────────────────────────────┤
│  HTTP Server (Axum on :8080)                                    │
│  ├── GET  /health           ──► Health check                    │
│  ├── POST /init             ──► Initialize kernel               │
│  ├── POST /query            ──► Run agent query                 │
│  ├── POST /model/load       ──► Load SVM model                  │
│  ├── POST /model/predict    ──► SVM prediction                  │
│  ├── GET  /models           ──► List models                     │
│  └── GET  /stream?query=... ──► WebSocket streaming             │
├─────────────────────────────────────────────────────────────────┤
│  Unikernel State (in-memory)                                    │
│  ├── KernelState (DB + Settings)                                │
│  ├── ToolRegistry (13 tools)                                    │
│  └── NullEventSink (no-op for headless)                        │
└─────────────────────────────────────────────────────────────────┘
```

## Component Diagram

```
┌────────────────────────────────────────────────────────────────────────────┐
│                           COMPONENT RELATIONSHIPS                          │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐                 │
│  │   Frontend  │     │   Commands  │     │  Piscis     │                 │
│  │  Services   │────►│  (Tauri)    │────►│  Kernel     │                 │
│  └─────────────┘     └─────────────┘     └─────────────┘                 │
│       │                    │                    │                         │
│       ▼                    ▼                    ▼                         │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐                 │
│  │ CoreEngine  │     │initialize_  │     │ HeadlessDeps│                 │
│  │ PiscisTools │     │core/        │     │  - DB       │                 │
│  │ Workbench   │     │piscis_tools │     │  - Settings │                 │
│  └─────────────┘     │execute_     │     │  - Tools    │                 │
│                      │piscis_tool  │     │  - EventSink│                 │
│       │              └─────────────┘     └──────┬──────┘                 │
│       ▼                                         │                         │
│  ┌─────────────┐                                ▼                         │
│  │   Tauri     │     ┌─────────────┐     ┌─────────────┐                 │
│  │   Events    │◄────│  Tauri      │     │ run_piscis_ │                 │
│  │   (Stream)  │     │   Window    │     │  _turn()    │                 │
│  └─────────────┘     └─────────────┘     └─────────────┘                 │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

## Data Flow: Agent Query Execution

```
User Input
    │
    ▼
┌────────────────────────────────────────────────────────────────────────┐
│  Frontend: coreEngine.runQuery("analyze this code")                   │
└────────────────────────────────────────────────────────────────────────┘
    │
    ▼ invoke('run_agent_query', { query })
┌────────────────────────────────────────────────────────────────────────┐
│  Backend: run_agent_query()                                            │
│  1. Get KernelState (DB, Settings)                                    │
│  2. Create ToolRegistry with 13 tools                                 │
│  3. Build HeadlessDeps with TauriEventSink                            │
│  4. Create HeadlessCliRequest with query                              │
│  5. Call run_piscis_turn(request, deps)                               │
└────────────────────────────────────────────────────────────────────────┘
    │
    ▼ piscis_kernel::headless::run_piscis_turn()
┌────────────────────────────────────────────────────────────────────────┐
│  Piscis Agent Loop                                                     │
│  1. Load LLM client (Anthropic/OpenAI from settings)                  │
│  2. Build system prompt + context                                      │
│  3. Create HarnessConfig with tools, policy, compaction               │
│  4. Run agent loop:                                                    │
│     - LLM generates tool calls                                         │
│     - ToolRegistry.execute() → Tool.call()                             │
│     - Results fed back to LLM                                          │
│     - Stream tokens via EventSink.emit_session()                       │
│  5. Persist messages to DB                                             │
│  6. Return HeadlessCliResponse                                         │
└────────────────────────────────────────────────────────────────────────┘
    │
    ▼ TauriEventSink.emit_session() → Window.emit("core-agent_event")
┌────────────────────────────────────────────────────────────────────────┐
│  Frontend: listen('core-agent_event', callback)                       │
│  - Receives: TextDelta, ToolUse, ToolResult, Done, Error              │
│  - Updates UI in real-time                                             │
└────────────────────────────────────────────────────────────────────────┘
```

## Tool Execution Flow

```
Frontend: piscisTools.executeTool({ tool_name: 'file_read', parameters: {path: 'x.rs'} })
    │
    ▼ invoke('execute_piscis_tool', request)
┌────────────────────────────────────────────────────────────────────────┐
│  Backend: execute_piscis_tool()                                        │
│  1. Get/create ToolContext for session                                │
│  2. Get KernelState → create ToolRegistry                             │
│  3. tool_registry.get('file_read') → &dyn Tool                        │
│  4. tool.call(input, &tool_context).await                             │
│  5. Emit events: piscis-tool-start → piscis-tool-complete/error       │
└────────────────────────────────────────────────────────────────────────┘
    │
    ▼ Tool.call() implementation (piscis-kernel)
┌────────────────────────────────────────────────────────────────────────┐
│  file_read tool:                                                       │
│  1. Resolve path within workspace_root                                │
│  2. Check permissions (bypass_permissions = true)                     │
│  3. Read file content                                                 │
│  4. Return ToolResult { content, is_error: false }                    │
└────────────────────────────────────────────────────────────────────────┘
```

## State Management

### Core Engine State (Rust)
```rust
struct CoreEngineState {
    kernel_state: Option<KernelState>,  // (Arc<Mutex<Database>>, Arc<Mutex<Settings>>)
    app_data_dir: Option<PathBuf>,
    session_cache: Mutex<HashMap<String, String>>,
}
static CORE_ENGINE: Lazy<Mutex<CoreEngineState>> = ...
```

### Tool Context State (Per Session)
```rust
struct ToolContext {
    session_id: String,
    workspace_root: PathBuf,
    bypass_permissions: bool,
    settings: Arc<ToolSettings>,
    max_iterations: Option<u32>,
    memory_owner_id: String,
    pool_session_id: Option<String>,
    tool_use_id: Option<String>,
    cancel: Arc<AtomicBool>,
    loop_halt: Option<Arc<AtomicBool>>,
}
static PISCIS_TOOL_CONTEXTS: Lazy<Mutex<HashMap<String, ToolContext>>> = ...
```

### Frontend State
```typescript
// CoreEngineService
- mode: 'local' | 'unikernel'
- initialized: boolean
- event listeners: Map<event, callback[]>

// PiscisToolsService
- initialized: boolean
- tool event listeners: Map<event, Set<callback>>
```

## Configuration

### Environment Variables
```bash
# Frontend
VITE_USE_UNIKERNEL=false          # true = use unikernel HTTP/WS
VITE_CORE_ENGINE_URL=http://localhost:8080

# Backend (piscis-engine settings via config.json)
# ~/.config/elixide/config.json:
{
  "provider": "anthropic",
  "model": "claude-3-5-sonnet-20241022",
  "api_key": "...",
  "workspace_root": "/path/to/workspace",
  "max_tokens": 8192,
  "context_window": 200000
}
```

### Cargo Features
```toml
[features]
default = ["desktop"]
desktop = [
    "tauri", "tauri-plugin-opener", "tauri-plugin-dialog",
    "tauri-plugin-fs", "tauri-plugin-updater", "tauri-plugin-store",
    "tauri-plugin-sql/sqlite", "tauri-plugin-clipboard-manager",
    "tauri-plugin-shell", "portable-pty", "parking_lot"
]
unikernel = []  # No Tauri, no GUI deps
```

## Security Model

### Permissions
- **Local Mode**: Full file system access within workspace (bypass_permissions = true)
- **Unikernel Mode**: File access restricted to mounted volumes
- **Shell**: Command allowlist via piscis policy gate
- **Network**: Web search/fetch via configured HTTP client

### Isolation
- Tauri: Process isolation via `tauri-plugin-isolation` (optional)
- Unikernel: Hardware virtualization (KVM) + Nanos single-process

## Performance Characteristics

| Metric | Local (Tauri) | Unikernel |
|--------|---------------|-----------|
| Binary Size | ~18 MB | ~18 MB |
| Startup Time | ~200 ms | ~100 ms |
| Memory (idle) | ~80 MB | ~30 MB |
| Agent Query | ~200 µs + LLM | ~200 µs + LLM |
| Tool Call | ~50-500 µs | ~50-500 µs |
| SVM Predict | ~72 ns | ~72 ns |

## Extension Points

### Adding New Piscis Tools
1. Add tool to `piscis-kernel/src/tools/`
2. Register in `register_default_cli_tools()`
3. Add to `list_piscis_tools()` in `piscis_tools.rs`
4. Add frontend helper in `piscis-tools.ts`

### Custom Event Sinks
```rust
// Implement piscis_core::host::EventSink
struct MyEventSink;
impl EventSink for MyEventSink {
    fn emit_session(&self, session_id: &str, event: &str, payload: Value) { ... }
    fn emit_broadcast(&self, event: &str, payload: Value) { ... }
}
```

### Custom Host Tools
```rust
// Implement piscis_core::host::HostTools
impl HostTools for MyHostTools {
    fn register(&self, registry: &mut ToolRegistryHandle) {
        registry.downcast_mut::<ToolRegistry>()
            .expect("kernel registry")
            .register(Box::new(MyCustomTool::new()));
    }
}
```

## Testing Strategy

### Unit Tests (Rust)
```bash
cargo test --lib
cargo test --test integration_tests
```

### Benchmarks
```bash
cargo bench --bench core_engine_bench
python3 benchmarks/scripts/benchmark.py vscode cursor elixide -o results.json
```

### E2E Tests (Frontend)
```bash
npm run test:e2e
```

## Deployment Checklist

### Local Development
- [ ] `npm run tauri dev` starts without errors
- [ ] `initialize_core()` returns success
- [ ] `run_agent_query()` works with API key
- [ ] Tool calls emit proper events
- [ ] WebSocket streaming works

### Unikernel
- [ ] Cross-compile: `cargo build --release --target x86_64-unknown-linux-musl --features unikernel`
- [ ] `ops image create` succeeds
- [ ] `ops run` starts HTTP server
- [ ] All endpoints respond correctly
- [ ] WebSocket streaming works
- [ ] Model load/predict works

### CI/CD
- [ ] Cargo check passes
- [ ] Cargo test passes
- [ ] Release build passes
- [ ] Benchmarks run
- [ ] Docker build for unikernel passes