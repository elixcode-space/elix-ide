# ElixirIDE Agent Commands

## Project Overview
ElixirIDE is an AI-powered agentic IDE built on VS Code workbench (monaco-vscode-api) inside Tauri, with a Nanos unikernel deployment mode.

## Key Commands

### Development
- `npm run tauri:dev` — Start Tauri dev server (Vite + Rust)
- `npm run dev` — Start Vite dev server only (port 1420)
- `npm run build` — Build frontend
- `npm run tauri:build` — Build production Tauri app
- `npm run lint` — Run ESLint
- `npm run test:tauri` — Run Tauri E2E test suite
- `npm run test:tauri:quick` — Run quick health-check tests

### Rust Backend
- `cd src-tauri && cargo check` — Check Rust build
- `cd src-tauri && cargo test` — Run Rust tests
- `cd src-tauri && cargo build` — Build Rust

### Testing
- `bash testing/tauri/run-tests.sh` — Run all E2E tests
- `bash testing/tauri/run-tests.sh --quick` — Health checks only
- Test scripts in `testing/tauri/tests/functional/`

### Unikernel
- `bash unikernel/build.sh` — Build Nanos unikernel image
- Requires `ops` (Nanos toolchain) and Docker for cross-compilation
- `cargo build --release --target x86_64-unknown-linux-musl --features unikernel`

### Extensions
- `bash scripts/build-extensions.sh` — Build all built-in extensions
- `bash scripts/run-extension-flow-test.sh` — Run extension install flow test

## Architecture
- **Frontend**: React/TypeScript with monaco-vscode-api
- **Backend**: Rust/Tauri with 17 commands
- **Core Engine**: piscis-engine (Rust AI agent framework)
- **Unikernel**: Nanos (HTTP server on :8080, headless mode)
- **Dual mode**: Tauri desktop (local) or Nanos unikernel (production/edge)

## Brand
- All code uses `elixide` / `ElixirIDE` naming (no legacy blink/becknide references)
- Repository: `elixcode-space/elix-ide`
- Website: `https://elixcode.space`
