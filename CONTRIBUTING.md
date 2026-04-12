# Contributing to CMAS

Thank you for your interest in contributing to **CMAS (Claude Multi-Account Switcher)**! This guide will help you get started.

## Prerequisites

- **macOS** (required — CMAS uses macOS Keychain for credential management)
- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) >= 1.77.2
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/) (`cargo install tauri-cli`)

## Getting Started

1. **Fork** the repository on GitHub.

2. **Clone** your fork locally:

   ```bash
   git clone https://github.com/<your-username>/CMAS.git
   cd CMAS
   ```

3. **Install dependencies:**

   ```bash
   npm install
   ```

4. **Run in development mode:**

   ```bash
   cargo tauri dev
   ```

## Project Structure

| Directory | Description |
|-----------|-------------|
| `src/` | Vue 3 frontend (TypeScript, Tailwind CSS 4, Pinia) |
| `src/components/` | Vue components organized by feature |
| `src/stores/` | Pinia state management |
| `src/i18n/` | Internationalization (Vietnamese, English) |
| `src-tauri/src/commands/` | Tauri command handlers |
| `src-tauri/src/services/` | Rust business logic (keychain, VSCode, usage tracking) |
| `src-tauri/src/models/` | Rust data models |

## Development Workflow

1. Create a new branch from `main`:

   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes and test locally with `cargo tauri dev`.

3. Ensure the project builds without errors:

   ```bash
   # Frontend type check
   npm run build

   # Full Tauri build
   cargo tauri build
   ```

4. Commit your changes with a clear message:

   ```bash
   git commit -m "feat: add xyz feature"
   ```

5. Push to your fork and open a Pull Request.

## Commit Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

| Prefix | Usage |
|--------|-------|
| `feat:` | New feature |
| `fix:` | Bug fix |
| `docs:` | Documentation changes |
| `style:` | Code style (formatting, no logic change) |
| `refactor:` | Code refactoring |
| `test:` | Adding or updating tests |
| `chore:` | Build process, dependencies, tooling |

## Code Style

### Frontend (Vue / TypeScript)

- Use Vue 3 Composition API with `<script setup lang="ts">`
- Use TypeScript — avoid `any` types
- Style with Tailwind CSS 4 utility classes
- Keep components small and focused

### Backend (Rust)

- Follow standard Rust conventions (`cargo fmt`, `cargo clippy`)
- Use `anyhow::Result` for error handling in services
- Keep Tauri commands thin — delegate logic to services

## Internationalization (i18n)

CMAS supports Vietnamese and English. When adding user-facing text:

1. Add translation keys to both `src/i18n/vi.ts` and `src/i18n/en.ts`.
2. Use `$t('key')` in templates or `useI18n()` in script setup.

## Reporting Bugs

Use the [Bug Report](https://github.com/boykioyb/CMAS/issues/new?template=bug_report.yml) issue template. Include:

- Steps to reproduce
- Expected vs actual behavior
- CMAS version and macOS version
- Logs or screenshots if applicable

## Suggesting Features

Use the [Feature Request](https://github.com/boykioyb/CMAS/issues/new?template=feature_request.yml) issue template.

## Code of Conduct

Be respectful and constructive. We are all here to build something useful together.

## License

By contributing, you agree that your contributions will be licensed under the same license as the project.
