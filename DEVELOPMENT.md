# ElixirIDE Development Guide

## Project Overview

ElixirIDE is an AI-powered agentic IDE built on VS Code workbench (monaco-vscode-api) inside Tauri, with a Nanos unikernel deployment mode.

- **Repository**: https://github.com/elixcode-space/elix-ide
- **Website**: https://elixcode.space
- **License**: MIT
- **Version**: v0.1.0 (Initial Launch)
- **Language**: TypeScript/React (Frontend), Rust (Backend)

## Table of Contents

1. [Quick Start](#quick-start)
2. [Architecture](#architecture)
3. [Project Structure](#project-structure)
4. [Frontend Development](#frontend-development)
5. [Backend Development](#backend-development)
6. [Commands](#commands)
7. [Unikernel](#unikernel)
8. [Testing](#testing)
9. [Extensions](#extensions)
10. [Brand Guidelines](#brand-guidelines)
11. [Deployment](#deployment)
12. [Future Development Tasks](#future-development-tasks)
13. [Troubleshooting](#troubleshooting)
14. [Resources](#resources)

---

## Quick Start

### Prerequisites

- **Node.js**: >= 16.9.1
- **Rust**: Latest stable
- **Target**: `x86_64-unknown-linux-musl` (for unikernel builds)
- **Nanos `ops`**: For unikernel image creation
- **Docker**: For cross-compilation on macOS

### Installation

```bash
# Clone the repository
git clone https://github.com/elixcode-space/elix-ide.git
cd elix-ide

# Install frontend dependencies
npm install

# Start development server
npm run tauri:dev
```

### Build

```bash
# Frontend only
npm run build

# Backend check
cd src-tauri && cargo check

# Full Tauri build
npm run tauri:build

# Unikernel build
cargo build --release --target x86_64-unknown-linux-musl --features unikernel
```

---

## Architecture

### Dual Deployment Modes

#### Mode 1: Local Development (Tauri Desktop)

- Frontend runs in a Tauri WebView window
- Backend provides 17 Tauri commands
- Full file system access within workspace
- AI assistant via piscis-engine

#### Mode 2: Production/Edge (Nanos Unikernel)

- HTTP server on port 8080 (headless mode)
- No GUI, API-only interface
- Hardware virtualization via KVM + Nanos
- All AI features available via REST/WebSocket API

### Component Overview

| Layer | Technology |
|-------|-----------|
| Frontend | React 19 + TypeScript + monaco-vscode-api |
| Backend | Rust + Tauri v2 |
| Core Engine | piscis-engine (AI agent framework) |
| Unikernel | Nanos (HTTP/WS server) |
| Database | SQLite (via piscis-kernel) |
| LSP | tower-lsp |
| Search | tantivy |
| SVM | libsvm-rs |

---

## Project Structure

```
elixide/
├── .github/                    # GitHub templates and workflows
│   ├── workflows/              # CI/CD (ci.yml, release.yml)
│   ├── ISSUE_TEMPLATE/         # Bug report, feature request
│   ├── PULL_REQUEST_TEMPLATE.md
│   ├── CODEOWNERS
│   ├── CODE_OF_CONDUCT.md
│   ├── FUNDING.yml
│   └── dependabot.yml          # Auto-update config
├── src/                        # Frontend source
│   ├── components/             # React components
│   │   ├── common/             # Shared components (logo, spinner, splash)
│   │   ├── pages/              # Page components
│   │   ├── vscode/             # VS Code-style UI components
│   │   └── ...
│   ├── services/               # Business logic and API services
│   │   ├── vscode/             # VS Code service implementations
│   │   ├── core-engine.ts      # Core engine service
│   │   ├── piscis-tools.ts     # Piscis tools integration
│   │   ├── tauriGitService.ts  # Git service
│   │   ├── tauriLspService.ts  # LSP service
│   │   └── ...
│   ├── extensions/             # Built-in extensions
│   │   ├── elixide-dark/       # Dark theme extension
│   │   └── ...
│   ├── assets/                 # Static assets
│   ├── styles/                 # Global styles
│   └── webviews/               # VS Code webview implementations
├── src-tauri/                  # Backend source
│   ├── src/
│   │   ├── commands/           # Tauri commands (17 total)
│   │   ├── services/           # Backend services
│   │   ├── dap/                # Debug Adapter Protocol
│   │   ├── git/                # Git integration (git2-rs)
│   │   ├── lsp/                # LSP client/server
│   │   ├── terminal/           # Terminal/PTY support
│   │   ├── test_server/        # Debug test server
│   │   └── ...
│   ├── Cargo.toml
│   └── tauri.conf.json
├── unikernel/                  # Nanos unikernel config
│   ├── build.sh
│   ├── config.json
│   ├── Dockerfile.nanos
│   └── elixide-core            # Binary (gitignored)
├── extensions/                 # Built-in extensions
│   └── elixide-office-custom-editors/  # DOCX/XLSX/PPTX editors
├── extensions-web/             # Web extension implementations
│   └── elixide-office-custom-editors/
├── testing/                    # E2E test suite
│   └── tauri/
│       ├── run-tests.sh
│       ├── lib/                # Test client library
│       └── tests/              # Test scripts
├── benchmarks/                 # Performance benchmarks
├── scripts/                    # Build and utility scripts
├── docs/                       # Documentation
│   ├── public/                 # Public docs
│   └── private/                # Internal docs
├── AGENTS.md                   # Kilo agent commands
├── CHANGELOG.md                # Version history
├── CONTRIBUTING.md             # Contribution guidelines
├── SECURITY.md                 # Security policy
├── ARCHITECTURE.md             # Architecture details
└── README.md                   # Project overview
```

---

## Frontend Development

### Key Concepts

- **monaco-vscode-api**: Full VS Code editor engine inside WebView
- **Tauri invoke**: Frontend-to-backend communication
- **Tauri listen**: Backend-to-frontend event streaming
- **VS Code Workbench**: Native VS Code UI components via monaco-vscode-api

### Adding a New Tauri Command

1. **Backend**: Add command in `src-tauri/src/commands/`
2. **Frontend**: Add invoke call in `src/services/`
3. **Register**: Ensure command is listed in command registry

### Frontend Services

Key services in `src/services/`:

| Service | Purpose |
|---------|---------|
| `core-engine.ts` | Core engine initialization and state |
| `piscis-tools.ts` | Piscis tool execution |
| `tauriGitService.ts` | Git operations |
| `tauriLspService.ts` | Language server protocol |
| `aiChat.ts` | AI chat integration |
| `aiProviderService.ts` | AI provider management |

---

## Backend Development

### Tauri Commands

17 Tauri commands are registered in `src-tauri/src/commands/`:

- `core_engine.rs` — Core engine initialization, kernel state
- `dap.rs` — Debug Adapter Protocol (breakpoints, stack trace, variables)
- `git.rs` — Git operations (status, diff, commit, branch)
- `lsp.rs` — Language Server Protocol (completion, hover, definitions)
- `piscis_tools.rs` — Piscis tool execution
- `search.rs` — Search and indexing (tantivy)
- `settings.rs` — User settings management
- `extension_management.rs` — Extension install/uninstall
- `file_tree.rs` — File tree operations
- `updater.rs` — Auto-update checks
- `telemetry.rs` — Analytics events
- + more...

### Adding a Backend Service

1. Create module in `src-tauri/src/services/` or `src-tauri/src/commands/`
2. Add function with `#[tauri::command]` attribute
3. Register in `src-tauri/src/main.rs` invoke handler
4. Call from frontend via `invoke('command_name', args)`

---

## Commands

### Development

```bash
npm run tauri:dev      # Start Tauri dev server (Vite + Rust)
npm run dev            # Vite dev server only (port 1420)
npm run build          # Production frontend build
npm run tauri:build    # Production Tauri build
npm run lint           # ESLint
```

### Rust

```bash
cd src-tauri && cargo check   # Check build
cd src-tauri && cargo test    # Run tests
cd src-tauri && cargo build   # Build
```

### Unikernel

```bash
bash unikernel/build.sh        # Build Nanos unikernel
cargo build --release --target x86_64-unknown-linux-musl --features unikernel
```

### Testing

```bash
npm run test:tauri             # Full E2E test suite
npm run test:tauri:quick       # Health checks only
bash testing/tauri/run-tests.sh     # Run all tests
bash testing/tauri/run-tests.sh --quick  # Quick health check
```

### Extensions

```bash
bash scripts/build-extensions.sh        # Build all extensions
bash scripts/run-extension-flow-test.sh # Extension install flow test
```

---

## Unikernel

### Building

```bash
# Cross-compile for Nanos
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl --features unikernel

# Create image (requires ops)
cd unikernel
ops image create elixide-core -c config.json
```

### Configuration

`unikernel/config.json`:

```json
{
  "Program": "./elixide-core",
  "Args": [],
  "Mounts": { "models": "/.local/models" },
  "Klibs": ["nanos/net", "nanos/kvm"]
}
```

### Running

```bash
ops run elixide-core -c config.json
```

### HTTP API (Unikernel Mode)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/init` | POST | Initialize kernel |
| `/query` | POST | Run agent query |
| `/model/load` | POST | Load SVM model |
| `/model/predict` | POST | SVM prediction |
| `/models` | GET | List models |
| `/stream?query=...` | GET | WebSocket streaming |

---

## Testing

### E2E Test Infrastructure

Tests are in `testing/tauri/`:

- `run-tests.sh` — Main test runner
- `lib/test-client.sh` — Test client library (HTTP client, assertions, UI helpers)
- `lib/utils.sh` — Utilities (app control, log analysis, reporting)
- `tests/functional/` — Functional test suites (health, workbench, editor, terminal, AI)

### Running Tests

```bash
# All tests
bash testing/tauri/run-tests.sh

# Quick health checks
bash testing/tauri/run-tests.sh --quick

# Specific suite
bash testing/tauri/tests/functional/01-health.sh
```

### Writing New Tests

1. Create script in `testing/tauri/tests/functional/`
2. Source `test-client.sh` for test functions
3. Use `test_*` prefix for test functions
4. Use assertion functions: `assert_equals`, `assert_json_equals`, `assert_contains`, etc.
5. Output `TEST_RESULTS:passed=N,failed=N,skipped=N` at end

---

## Extensions

### Built-in Extensions

Extensions are in `extensions/builtin/elixide-office-custom-editors/`:

- DOCX editor
- XLSX editor
- PPTX editor

### Building Extensions

```bash
bash scripts/build-extensions.sh
```

### Extension Structure

```
extension/
├── package.json      # Extension manifest
├── extension.ts      # Main entry point
├── build.mjs         # Build script
└── README.md         # Extension docs
```

### Development Workflow

1. Create extension directory under `extensions/builtin/elixide-*`
2. Add `package.json` with extension manifest
3. Implement `extension.ts` with VS Code extension API
4. Add to extension registry in backend
5. Build: `bash scripts/build-extensions.sh`

---

## Brand Guidelines

### Correct Naming

| Correct | Wrong |
|---------|-------|
| ElixirIDE | ElixirIDE (no extra 'r') |
| elixide | elixide |
| ElixirIDE Code Assist | — |
| elixide-core (binary) | — |
| com.elixcode.elixide (identifier) | — |

### Areas to Check

- Source code comments and strings
- Configuration files (Cargo.toml, package.json, tauri.conf.json)
- Documentation (README.md, ARCHITECTURE.md, docs/)
- CI/CD workflows
- Script files
- Asset filenames

---

## Deployment

### Local Development

```bash
npm run tauri:dev
```

### Production Builds

```bash
npm run tauri:build
```

Outputs in `src-tauri/target/*/release/bundle/`:
- macOS: `.dmg`
- Windows: `.msi`
- Linux: `.AppImage`, `.deb`, `.rpm`

### Unikernel Deployment

```bash
# Build
bash unikernel/build.sh

# Run locally
ops run unikernel/elixide-core -c unikernel/config.json

# Deploy to cloud
# Upload unikernel image to provider
# Run with Nanos runtime
```

### CI/CD

GitHub Actions workflow in `.github/workflows/release.yml`:

- Builds on tag push (`v*`)
- Multi-platform: macOS (Intel + ARM), Windows, Linux
- Signed and notarized artifacts
- Auto-update manifest generation

### Release Process

1. Update version in `src-tauri/Cargo.toml` and `package.json`
2. Run `npm run tauri:build` for testing
3. Create tag: `git tag v0.1.1`
4. Push tag: `git push origin v0.1.1`
5. GitHub Actions builds and creates release automatically

---

## Future Development Tasks

These are planned features tracked in `tasks.txt`:

### Task 7: Extension Host Architecture (In Progress)
- Node.js child process for extension host
- RPC over stdio communication
- Extension lifecycle management

### Task 8: Extension API Surface
- Commands registration
- Views (tree views, webviews)
- Configuration API
- Menu contributions

### Task 9: Extension Management
- Open-VSX registry integration
- Install/uninstall flows
- Activation events
- Extension state management

### Task 10: DAP Client
- CodeLLDB for Rust/C++ debugging
- node-debug2 for JavaScript/TypeScript debugging
- Breakpoint management
- Variable inspection

### Task 11: Git Integration
- git2-rs based operations
- Status, diff, commit, push, branch, blame
- Git status panel
- Inline diff view

### Task 12: Terminal 2.0
- Shell integration
- CWD tracking
- Link detection
- Tabs and split panes

### Task 13: Packaging
- .dmg (macOS), .msi (Windows), .deb/.AppImage (Linux)
- Code signing and notarization
- Auto-update metadata

### Task 14: Auto-Update
- Delta updates
- Rollback support
- Release channels (stable, beta, dev)

### Task 15: Telemetry/Crash Reporting
- Sentry integration
- Opt-in analytics
- Error reporting
- Usage metrics

### Task 16: Dogfood & Polish
- Internal usage testing
- Bug bash sessions
- Documentation polish
- Performance optimization

---

## Troubleshooting

### Common Issues

**Build fails with "tauri not found"**
```bash
npm install  # Reinstall dependencies
```

**Cargo check fails**
```bash
cd src-tauri
cargo clean
cargo check
```

**Frontend build fails**
```bash
rm -rf node_modules web
npm install
npm run build
```

**Port 1420 already in use**
```bash
lsof -i :1420 | grep LISTEN
kill <PID>
```

**Test server not starting**
```bash
bash testing/tauri/lib/utils.sh stop_app
bash testing/tauri/run-tests.sh --quick
```

**Unikernel build fails**
```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl --features unikernel
```

---

## Resources

### Documentation

- **AGENTS.md**: Kilo agent commands reference
- **ARCHITECTURE.md**: Detailed architecture diagrams and data flow
- **CHANGELOG.md**: Version history and release notes
- **SECURITY.md**: Security policy and reporting
- **CONTRIBUTING.md**: Contribution guidelines
- **SETUP.md**: Launch setup guide

### External Links

- **GitHub**: https://github.com/elixcode-space/elix-ide
- **Website**: https://elixcode.space
- **Issues**: https://github.com/elixcode-space/elix-ide/issues
- **Discussions**: https://github.com/elixcode-space/elix-ide/discussions

### Related Projects

- **piscis-engine**: https://github.com/njbinbin-piscis/piscis-engine (Core AI engine)
- **Nanos**: https://nanos.app (Unikernel runtime)
- **Tauri**: https://tauri.app (Desktop framework)
- **monaco-vscode-api**: https://github.com/CodinGame/monaco-vscode-api
- **Open VSX**: https://open-vsx.org/ (Extension registry)

---

## Key Configuration Files

| File | Purpose |
|------|---------|
| `package.json` | Frontend dependencies and scripts |
| `src-tauri/Cargo.toml` | Rust dependencies and features |
| `src-tauri/tauri.conf.json` | Tauri configuration |
| `.cargo/config.toml` | Cargo build configuration |
| `.github/workflows/ci.yml` | CI pipeline |
| `.github/workflows/release.yml` | Release automation |
| `.github/dependabot.yml` | Auto-update config |
| `unikernel/config.json` | Nanos unikernel config |

---

*For questions or issues, open a GitHub Issue or Discussion.*
