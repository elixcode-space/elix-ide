# ElixirIDE Changelog

## v0.1.0 — Initial Launch

### Rebranding
- Full rebrand from `blink`/`becknide` to `elixide`/`ElixirIDE` across entire codebase
- Renamed packages: `blink-office-custom-editors` → `elixide-office-custom-editors`
- Renamed binary: `becknide-core` → `elixide-core`
- Updated all documentation, scripts, configs, and source code
- Brand assets: `blink-editor.png` → `elixide-editor.png`, `blink-extensions.png` → `elixide-extensions.png`

### Architecture
- **Frontend**: React/TypeScript with monaco-vscode-api
- **Backend**: Rust/Tauri with 17 commands
- **Core Engine**: piscis-engine (Rust AI agent framework)
- **Unikernel**: Nanos (HTTP server on :8080, headless mode)
- **Dual mode**: Tauri desktop (local) or Nanos unikernel (production/edge)

### New Features
- Extension Host Architecture (Node.js child process, RPC over stdio)
- Extension API Surface (commands, views, tree views, webviews, configuration)
- Extension Management (Open-VSX registry, install/uninstall, activation events)
- DAP Client (CodeLLDB for Rust/C++, node-debug2 for JS/TS)
- Git Integration (git2-rs for status, diff, commit, push, branch, blame)
- Terminal 2.0 (Shell integration, cwd tracking, links, tabs, split panes)
- Auto-Update (Delta updates, rollback, channels)
- Telemetry/Crash Reporting (Sentry, opt-in analytics)

### Build & Test
- Rust build passes (`cargo check`)
- Frontend build passes (`npm run build`)
- E2E test infrastructure in `testing/tauri/`
- Test scripts validated for syntax correctness

### DevOps
- Release workflow: `.github/workflows/release.yml` (macOS, Windows, Linux)
- AGENTS.md with project commands for Kilo agent system
- .gitignore configured for env files, unikernel binaries, secrets

---

## Prior: blink/becknide era
- Original fork from `bmarti44/blink`
