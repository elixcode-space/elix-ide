# ElixirIDE Launch Setup Guide

## 1. GitHub Secrets (Required for CI/CD)

Navigate to: `Settings → Secrets and variables → Actions`

### Required Secrets:
- `TAURI_PRIVATE_KEY` — Tauri signing key for builds
- `TAURI_KEY_PASSWORD` — Password for Tauri key
- `APPLE_CERTIFICATE` — macOS signing certificate (base64)
- `APPLE_CERTIFICATE_PASSWORD` — Certificate password
- `APPLE_SIGNING_IDENTITY` — macOS signing identity
- `APPLE_ID` — Apple Developer account ID
- `APPLE_PASSWORD` — Apple Developer account password
- `APPLE_TEAM_ID` — Apple Developer team ID
- `WINDOWS_CERTIFICATE` — Windows signing certificate (base64)
- `WINDOWS_CERTIFICATE_PASSWORD` — Windows certificate password

### Optional Secrets:
- `GITHUB_TOKEN` — Auto-provided by GitHub Actions

## 2. Branch Protection (Recommended)

Navigate to: `Settings → Branches → Branch protection rules → Add rule`

### Settings for `main` branch:
- ✅ Require a pull request before merging
- ✅ Require approvals: 1
- ✅ Dismiss stale PR reviews
- ✅ Require conversation to be resolved
- ✅ Require linear history
- ❌ Allow force pushes
- ❌ Allow deletions
- ✅ Include administrators (optional)

## 3. Repository Topics

Navigate to: `Settings → General → Topics`

Suggested topics:
- elixide
- tauri
- rust
- ide
- vscode
- monaco
- ai-assistant
- unikernel

## 4. Enable Discussions

Navigate to: `Settings → General → Features → Discussions`

## 5. Project Board

Navigate to: `Projects tab → Create new project`

Suggested columns:
- Backlog
- In Progress
- Review
- Done

## 6. Invite Collaborators

Navigate to: `Settings → Collaborators → Add people`

## 7. Configure GitHub Pages (Optional)

Navigate to: `Settings → Pages → Source`

## 8. Enable Dependabot Security Updates

Navigate to: `Settings → Code security and analysis → Enable`

## 9. Set Up Secret Scanning

Navigate to: `Settings → Code security and analysis → Secret scanning`

## 10. Configure Notifications

Recommended: Watch the repository for release notifications
