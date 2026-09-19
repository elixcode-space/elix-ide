# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| v0.1.0  | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it to us privately:

- **Email**: maintainers@elixcode.space
- **GitHub Security Advisory**: https://github.com/elixcode-space/elix-ide/security/advisories/new

Please do not open public issues for security vulnerabilities.

## Response Process

1. We acknowledge receipt within 48 hours
2. We investigate and assess the severity
3. We develop a fix and coordinate disclosure
4. We release a patched version
5. We publish a security advisory

## Security Best Practices

- All dependencies are regularly audited (`npm audit`, `cargo audit`)
- User data is processed locally (no telemetry by default)
- Unikernel mode provides hardware-level isolation
