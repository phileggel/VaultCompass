#!/usr/bin/env bash
set -euo pipefail

# sync-config.sh — Stable bootstrap entry point for claude-kit.
#
# Copy this file once to scripts/sync-config.sh in your downstream project.
# This file is intentionally minimal — all sync logic lives in kit/scripts/sync.sh.
# After cloning the kit, this script self-updates from the kit if the bootstrap
# itself has changed, then re-executes the new version.
#
# Usage:
#   ./scripts/sync-config.sh            # sync latest release tag
#   ./scripts/sync-config.sh v4.0.0     # sync a specific tag
#   ./scripts/sync-config.sh -y         # overwrite drifted docs without prompting
#   ./scripts/sync-config.sh -h         # show help and exit

REPO="https://github.com/phileggel/claude-kit"
# Colors (respect NO_COLOR=1)
if [ -n "${NO_COLOR:-}" ]; then
    BLUE='' YELLOW='' NC=''
else
    BLUE='\033[0;34m'
    YELLOW='\033[0;33m'
    NC='\033[0m'
fi

# Capture original args before parsing so we can re-exec with them after self-update
ORIG_ARGS=("$@")

VERSION=""
KIT_SYNC_YES="false"

_print_help() {
    cat <<'EOF'
sync-config.sh — Sync claude-kit into a downstream project.

Usage:
  ./scripts/sync-config.sh              sync latest release tag
  ./scripts/sync-config.sh vX.Y.Z       sync a specific tag
  ./scripts/sync-config.sh -y           overwrite drifted docs without prompting
  ./scripts/sync-config.sh -h           show this help and exit

Flags:
  -y, --yes      auto-answer "yes" to the docs-drift overwrite prompt
  -h, --help     show this help and exit

Environment:
  SYNC_NO_HOOKS=1   skip auto-activation of core.hooksPath = .githooks
  NO_COLOR=1        disable ANSI colors
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
    --profile=* | --profile)
        echo -e "${YELLOW}⚠  --profile is deprecated and ignored — claude-kit is Tauri-only as of v4.0.0${NC}"
        # Skip the value too if --profile was given as a separate arg
        if [[ "$1" == "--profile" ]] && [[ $# -ge 2 ]]; then shift; fi
        shift
        ;;
    -y | --yes)
        KIT_SYNC_YES="true"
        shift
        ;;
    -h | --help)
        _print_help
        exit 0
        ;;
    -*)
        echo -e "${YELLOW}✗ Unknown flag: $1${NC}" >&2
        echo "Run with -h for usage." >&2
        exit 2
        ;;
    *)
        VERSION="$1"
        shift
        ;;
    esac
done

if [[ -z "$VERSION" ]]; then
    # Pick the latest release tag (vX.Y.Z).
    VERSION=$(git ls-remote --tags --sort="v:refname" "$REPO" |
        sed 's/.*\///; s/\^{}//' |
        grep -E '^v[0-9]' |
        tail -n1)
    if [[ -z "$VERSION" ]]; then
        echo -e "${YELLOW}⚠  No release tag found. Pass the version explicitly.${NC}" >&2
        exit 1
    fi
fi

TMP=$(mktemp -d)
# $TMP is NOT cleaned here — ownership passes to sync.sh via KIT_TMP

echo -e "${BLUE}⬇  Cloning claude-kit@${VERSION}...${NC}"
git clone --depth 1 --branch "$VERSION" "$REPO" "$TMP" --quiet \
    -c core.autocrlf=false

# ── Self-update (only on first invocation; KIT_SELF_UPDATED guards against loops) ──
if [[ -z "${KIT_SELF_UPDATED:-}" ]]; then
    SELF="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)/$(basename "${BASH_SOURCE[0]}")"
    KIT_BOOTSTRAP="$TMP/kit/sync-config.sh"
    if [[ -f "$KIT_BOOTSTRAP" ]] && ! cmp -s "$KIT_BOOTSTRAP" "$SELF"; then
        echo -e "${BLUE}🔄 Bootstrap updated in kit — refreshing $SELF and re-running...${NC}"
        cp "$KIT_BOOTSTRAP" "$SELF"
        chmod +x "$SELF"
        rm -rf "$TMP"
        export KIT_SELF_UPDATED=true
        exec "$SELF" "${ORIG_ARGS[@]}"
    fi
fi

export KIT_TMP="$TMP"
export KIT_SYNC_YES
exec bash "$TMP/kit/scripts/sync.sh" "$VERSION"
