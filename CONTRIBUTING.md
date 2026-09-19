# Contributing to ElixirIDE

Thank you for your interest in contributing!

## How to Contribute

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Development Setup

### Prerequisites

- Node.js >= 16.9.1
- Rust with target `x86_64-unknown-linux-musl` (for unikernel)
- Nanos `ops` tool (for unikernel builds)

### Build

```bash
# Frontend
npm run build

# Backend
cd src-tauri && cargo check

# Unikernel
cargo build --release --target x86_64-unknown-linux-musl --features unikernel
```

## Pull Request Guidelines

- Use conventional commit messages (feat:, fix:, docs:, refactor:, etc.)
- Update documentation as needed
- Ensure all tests pass
- Update CHANGELOG.md

## Questions?

Open an issue or discussion on GitHub.
