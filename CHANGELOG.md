# Changelog

## v0.1.1 — 2026-04-07

### Isolated VSCode Sessions
- Each account runs VSCode in a **separate process** with its own keychain entry
- Uses `--user-data-dir` to force VSCode to create a new Electron process (instead of IPC to an existing one)
- Uses `--extensions-dir` to share already-installed extensions
- Automatically copies `settings.json` and `keybindings.json` from default VSCode on first launch
- Switching accounts in CMAS **does not affect** already-open VSCode windows

### Credential Fixes
- Fix `security -i` writing incorrect hex data (removed `"` wrapping hex in `-X` flag)
- Each VSCode session uses its own keychain entry (`-a cmas-{account_id}`) instead of global username
- "Open VSCode" no longer changes the global active account — only creates an isolated session

### Usage Display
- Dashboard automatically scans session JSONL files when loading account list (previously usage was always 0)
- Display **session reset time**: countdown until the 5h window expires (e.g. "Resets in 2h 15m")
- Display **weekly reset time**: weekly reset date + countdown (e.g. "Resets in 3d (Mon 04/07)")
- Display detailed output tokens + message count within the 5h session
- Per-model breakdown: separate Opus and Sonnet usage

### Window Drag
- Added `core:window:allow-start-dragging` permission to Tauri capabilities
- App window can now be dragged normally

---

### Technical Details

**Session isolation architecture:**
```
Open VSCode for account A:
  → keychain: write to -a "cmas-88c7f287" -s "Claude Code-credentials"
  → vscode:   code --new-window --user-data-dir ~/.claude-switcher/vscode-sessions/cmas-88c7f287/
                    --extensions-dir ~/.vscode/extensions/
  → env:      USER=cmas-88c7f287

Open VSCode for account B:
  → keychain: write to -a "cmas-13a719ea" -s "Claude Code-credentials"  
  → vscode:   code --new-window --user-data-dir ~/.claude-switcher/vscode-sessions/cmas-13a719ea/
                    --extensions-dir ~/.vscode/extensions/
  → env:      USER=cmas-13a719ea
```

Each VSCode process reads keychain from its own entry → fully independent.

**Files changed:**
- `src-tauri/src/services/keychain.rs` — Added `write_session_credentials()`, refactored `write_credentials_for_user()`
- `src-tauri/src/services/vscode.rs` — Added `--user-data-dir`, `--extensions-dir`, `USER` env var, auto-copy settings
- `src-tauri/src/services/usage_tracker.rs` — Added `session_reset_at`, `weekly_reset_at`, track earliest message in 5h window
- `src-tauri/src/commands/switch.rs` — `switch_and_open_vscode` uses isolated session, no longer changes global active
- `src-tauri/src/commands/account.rs` — `list_accounts` automatically refreshes usage for active account
- `src-tauri/src/models/account.rs` — Added `session_reset_at`, `weekly_reset_at` to `UsageInfo`
- `src-tauri/capabilities/default.json` — Added `core:window:allow-start-dragging`
- `src/types/index.ts` — Added `session_reset_at`, `weekly_reset_at`
- `src/components/dashboard/CurrentAccount.vue` — Display reset countdown, token details
- `src/i18n/vi.ts`, `src/i18n/en.ts` — Added key `resetNow`
