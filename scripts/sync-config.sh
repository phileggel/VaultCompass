#!/usr/bin/env bash
set -euo pipefail

# sync-config.sh — Pull tauri-claude-kit into the current project
#
# Usage:
#   ./scripts/sync-config.sh          # pulls latest main
#   ./scripts/sync-config.sh v1.2.0   # pulls a specific tag

REPO="https://github.com/phileggel/tauri-claude-kit"
VERSION="${1:-main}"

YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

PROJECT_ROOT="$(git rev-parse --show-toplevel)"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

echo -e "${BLUE}⬇  Cloning tauri-claude-kit@${VERSION}...${NC}"
git clone --depth 1 --branch "$VERSION" "$REPO" "$TMP" --quiet

# EXTRACTION DE LA VERSION RÉELLE
# Si c'est un tag, on garde le nom du tag. Si c'est 'main', on prend le SHA court.
pushd "$TMP" > /dev/null
REAL_VERSION=$(git describe --tags --exact-match 2>/dev/null || git rev-parse --short HEAD)
popd > /dev/null

echo -e "${BLUE}📁 Syncing agents...${NC}"
mkdir -p "$PROJECT_ROOT/.claude/agents"
cp "$TMP/agents/"*.md "$PROJECT_ROOT/.claude/agents/"

echo -e "${BLUE}📁 Syncing skills...${NC}"
# Utilisation de find pour éviter les erreurs si le dossier est vide
find "$TMP/skills/" -maxdepth 1 -type d -not -path "$TMP/skills/" | while read -r skill_dir; do
  skill_name=$(basename "$skill_dir")
  mkdir -p "$PROJECT_ROOT/.claude/skills/$skill_name"
  cp "$skill_dir/SKILL.md" "$PROJECT_ROOT/.claude/skills/$skill_name/"
done

echo -e "${BLUE}📁 Syncing scripts...${NC}"
cp "$TMP/scripts/check.py" "$PROJECT_ROOT/scripts/"
cp "$TMP/scripts/release.py" "$PROJECT_ROOT/scripts/"
cp "$TMP/scripts/sync-config.sh" "$PROJECT_ROOT/scripts/"

echo -e "${BLUE}📁 Syncing .githooks...${NC}"
mkdir -p "$PROJECT_ROOT/.githooks"
cp "$TMP/.githooks/commit-msg" "$PROJECT_ROOT/.githooks/"
cp "$TMP/.githooks/pre-commit" "$PROJECT_ROOT/.githooks/"
cp "$TMP/.githooks/pre-push" "$PROJECT_ROOT/.githooks/"
cp "$TMP/.githooks/README.md" "$PROJECT_ROOT/.githooks/"

echo -e "${BLUE}📁 Syncing common justfile...${NC}"
cp "$TMP/.justfile-common" "$PROJECT_ROOT/.justfile-common"

echo "$REAL_VERSION" > "$PROJECT_ROOT/.claude-kit-version"

echo -e "${GREEN}✅ Synced tauri-claude-kit@${REAL_VERSION}${NC}"
echo -e "${YELLOW}→ Review changes before committing (git diff).${NC}"
