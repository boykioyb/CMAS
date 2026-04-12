<p align="center">
  <img src="src-tauri/icons/icon.png" alt="CMAS Logo" width="120" />
</p>

<h1 align="center">CMAS — Claude Multi Account Switcher</h1>

<p align="center">
  Manage and switch between multiple Claude Code accounts on macOS.<br/>
  Isolated VSCode sessions. Usage tracking. Zero re-login.
</p>

<p align="center">
  <a href="https://creativecommons.org/licenses/by-nc-sa/4.0/">
    <img src="https://img.shields.io/badge/license-CC%20BY--NC--SA%204.0-blue.svg" alt="License" />
  </a>
  <img src="https://img.shields.io/badge/platform-macOS-lightgrey.svg" alt="Platform" />
  <img src="https://img.shields.io/badge/built%20with-Tauri%202%20%2B%20Vue%203-blueviolet.svg" alt="Built with" />
</p>

---

## Screenshots

<p align="center">
  <img src="docs/screenshots/accounts.png" alt="Account Management" width="720" />
  <br/><em>Account Management — manage multiple Claude accounts with usage tracking</em>
</p>

<p align="center">
  <img src="docs/screenshots/cost_usage.png" alt="Cost Usage" width="720" />
  <br/><em>Cost Usage — daily cost charts, model breakdown, and cumulative spending</em>
</p>

## Features

- **Multi-account management** — Add, edit, delete Claude accounts with an intuitive interface
- **Quick switching** — Switch credentials via macOS Keychain, no re-login required
- **VSCode Session Isolation** — Each account opens VSCode in a separate process, preventing credential conflicts
- **Cost & Usage tracking** — API cost estimation, daily charts, per-model breakdown (Opus/Sonnet/Haiku)
- **Session & weekly reset** — Countdown timers for 5h session reset and weekly quota reset
- **Optimal account suggestion** — Suggest the account with the lowest usage
- **Multilingual** — English and Vietnamese

## Tech Stack

| Layer | Technology |
|-------|------------|
| Frontend | Vue 3, TypeScript, Tailwind CSS 4, Pinia, Vue Router |
| Backend | Rust, Tauri 2 |
| Credential | macOS Keychain (`security-framework`) |
| Storage | Tauri Plugin Store (JSON) |

## Getting Started

### Requirements

- **macOS** (uses Keychain for credential management)
- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) >= 1.77.2
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/)

### Installation

```bash
git clone https://github.com/boykioyb/CMAS.git
cd CMAS
npm install
```

### Development

```bash
cargo tauri dev
```

### Build

```bash
cargo tauri build
```

## How It Works

1. **Store credentials** — Each account saves its OAuth token to macOS Keychain with a unique entry (`cmas-{id}`)
2. **Switch account** — Writes the selected account's credential to the Keychain entry that Claude Code reads
3. **VSCode isolation** — Opens VSCode with `--user-data-dir` per account, ensuring fully independent sessions
4. **Usage tracking** — Scans Claude Code's JSONL files to calculate token usage and estimate API costs

## Project Structure

```
CMAS/
├── src/                          # Frontend (Vue 3)
│   ├── components/
│   │   ├── accounts/             # AccountGrid, AccountTable, Add/Edit dialogs
│   │   ├── common/               # Navbar, Toast, Dialog, ProgressBar
│   │   └── dashboard/            # CurrentAccount, StatsCards, BestAccountSuggestion
│   ├── pages/                    # Dashboard, Accounts, Cost Usage, Settings
│   ├── stores/                   # Pinia stores (account, config, ui, costUsage)
│   ├── i18n/                     # English, Vietnamese
│   └── types/                    # TypeScript interfaces
├── src-tauri/                    # Backend (Rust + Tauri 2)
│   ├── src/
│   │   ├── commands/             # Tauri commands (account, switch, auth, config, quota)
│   │   ├── models/               # Data models (Account, Config)
│   │   └── services/             # Business logic
│   │       ├── keychain.rs       # macOS Keychain read/write
│   │       ├── vscode.rs         # VSCode session isolation
│   │       ├── usage_tracker.rs  # Parse JSONL, calculate usage
│   │       ├── claude_auth.rs    # Claude authentication
│   │       └── claude_config.rs  # Claude config management
│   └── capabilities/             # Tauri permission config
└── docs/                         # Screenshots & documentation
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to contribute.

## License

This project is licensed under [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/) — see the [LICENSE](LICENSE) file for details.

### Trademark

"CMAS" and "Claude Multi Account Switcher" are trademarks of Hoa TQ. See [TRADEMARK.md](TRADEMARK.md) for usage guidelines.

## Author

**Hoa TQ** — [GitHub](https://github.com/boykioyb) · [hoatq.dev](https://hoatq.dev)
