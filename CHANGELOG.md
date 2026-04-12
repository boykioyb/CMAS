# Changelog

All notable changes to this project will be documented in this file.

## [1.1.0] — 2026-04-13

### Features

- **Windows support** — CMAS now builds and runs on Windows in addition to macOS ([#1](https://github.com/boykioyb/CMAS/issues/1))
- **CI/CD pipeline** — GitHub Actions workflow builds for macOS (ARM64 + x64), Windows x64, and Linux x64 on tag push
- **Release script** — `scripts/release.sh` bumps version across all config files, commits, tags, and pushes in one command

### Changes

- **Credential storage** — Replaced macOS `security` CLI with [`keyring`](https://crates.io/crates/keyring) crate for cross-platform Keychain/Credential Manager support
- **VSCode isolation (Windows)** — Uses NTFS junctions instead of Unix symlinks; sets `USERPROFILE`/`USERNAME` instead of `HOME`/`USER`
- **CLI discovery** — `find_vscode_path()` and `find_claude_cli()` now detect Windows install paths (`%LOCALAPPDATA%`, `%ProgramFiles%`)
- **Browser open** — `open_claude_login` uses platform-appropriate command (`open` / `cmd /C start` / `xdg-open`)
- **Path handling** — Fixed `path_to_project_dir_name` to handle Windows backslash separators
- **i18n** — Removed macOS-specific wording from descriptions

### Technical

- Removed unused `security-framework` dependency
- Added `keyring` (cross-platform) and `junction` (Windows) dependencies
- Conditional compilation via `#[cfg(target_os)]` throughout services layer

---

## [1.0.0] — 2026-04-12

### Features

- **Multi-account management** — Add, edit, and delete Claude Code accounts with search, filtering, and grid/table views
- **One-click switching** — Switch credentials via macOS Keychain instantly, no re-login required
- **VSCode session isolation** — Each account opens VSCode in a separate Electron process with its own `--user-data-dir`, preventing credential conflicts between sessions
- **Cost & usage analytics** — Estimate API costs, view daily spending charts, per-model breakdown (Opus/Sonnet/Haiku), and cumulative cost tracking
- **Session & weekly reset timers** — Countdown for 5-hour session window and weekly quota reset
- **Optimal account suggestion** — Automatically suggest the account with the lowest usage
- **OAuth usage scraping** — Fetch real-time usage data from Claude's API per account
- **Dark mode** — System-aware theme with manual light/dark/system toggle
- **Multilingual** — English and Vietnamese with persistent language preference
- **Settings management** — Configurable VSCode path, quota warning threshold, usage refresh interval

### Technical Highlights

- **Credential security** — All tokens stored in macOS Keychain with per-account entries (`cmas-{id}`), never in plain text
- **Session isolation architecture** — Each VSCode instance runs with isolated `--user-data-dir`, shared `--extensions-dir`, and unique `USER` env var
- **Auto-copy VSCode settings** — First launch copies `settings.json` and `keybindings.json` from default VSCode profile
- **JSONL usage parser** — Scans Claude Code session files to calculate per-model token usage
- **Chrome cookie extraction** — Decrypt Chrome cookies via PBKDF2-SHA1 for OAuth token retrieval

### Tech Stack

- **Frontend** — Vue 3, TypeScript, Tailwind CSS 4, Pinia, Vue Router, Chart.js
- **Backend** — Rust, Tauri 2
- **Credential** — [`keyring`](https://crates.io/crates/keyring) (macOS Keychain / Windows Credential Manager)
- **Storage** — Tauri Plugin Store (JSON)
