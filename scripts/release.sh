#!/usr/bin/env bash
set -euo pipefail

# ── Usage ────────────────────────────────────────────────────────
# ./scripts/release.sh <version>
#
# Examples:
#   ./scripts/release.sh 1.1.0
#   ./scripts/release.sh 2.0.0-beta.1
# ─────────────────────────────────────────────────────────────────

NEW_VERSION="${1:-}"

if [ -z "$NEW_VERSION" ]; then
  echo "Usage: $0 <version>"
  echo "  e.g. $0 1.1.0"
  exit 1
fi

# Strip leading 'v' if provided (e.g. v1.1.0 -> 1.1.0)
NEW_VERSION="${NEW_VERSION#v}"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "Updating version to $NEW_VERSION ..."

# ── 1. package.json ──────────────────────────────────────────────
npm version "$NEW_VERSION" --no-git-tag-version

# ── 2. src-tauri/tauri.conf.json ────────────────────────────────
if command -v jq &>/dev/null; then
  tmp=$(mktemp)
  jq --arg v "$NEW_VERSION" '.version = $v' src-tauri/tauri.conf.json > "$tmp"
  mv "$tmp" src-tauri/tauri.conf.json
else
  sed -i.bak -E "s/\"version\": \"[^\"]+\"/\"version\": \"$NEW_VERSION\"/" src-tauri/tauri.conf.json
  rm -f src-tauri/tauri.conf.json.bak
fi

# ── 3. src-tauri/Cargo.toml ─────────────────────────────────────
sed -i.bak -E "s/^version = \"[^\"]+\"/version = \"$NEW_VERSION\"/" src-tauri/Cargo.toml
rm -f src-tauri/Cargo.toml.bak

# ── 4. Cargo.lock (regenerate from Cargo.toml) ──────────────────
(cd src-tauri && cargo update -p app --precise "$NEW_VERSION" 2>/dev/null || cargo generate-lockfile)

# ── Verify ───────────────────────────────────────────────────────
echo ""
echo "Updated versions:"
echo "  package.json:       $(node -p "require('./package.json').version")"
echo "  tauri.conf.json:    $(node -p "require('./src-tauri/tauri.conf.json').version")"
echo "  Cargo.toml:         $(grep '^version' src-tauri/Cargo.toml | head -1)"
echo ""

# ── Git commit + tag + push ──────────────────────────────────────
read -rp "Commit, tag v$NEW_VERSION, and push? [y/N] " confirm
if [[ "$confirm" =~ ^[Yy]$ ]]; then
  git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
  git commit -m "chore: bump version to $NEW_VERSION"
  git tag "v$NEW_VERSION"
  git push origin main
  git push origin "v$NEW_VERSION"
  echo ""
  echo "Done! v$NEW_VERSION pushed — CI will build & create a draft release."
else
  echo ""
  echo "Files updated but NOT committed. Run manually:"
  echo "  git add -A && git commit -m 'chore: bump version to $NEW_VERSION'"
  echo "  git tag v$NEW_VERSION && git push origin main --tags"
fi
