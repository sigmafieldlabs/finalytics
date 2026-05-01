#!/bin/bash
# post_commit.sh — Run after committing to GitHub.
#
# Steps:
#   1. Publishes the Rust crate to crates.io         (cargo publish)
#   2. Publishes the JS package to npm               (npm publish)
#   3. Builds and publishes the C-FFI GitHub release  (ffi/release_ffi.sh)
#   4. Creates and pushes the Python package git tag  (python/vX.Y.Z)
#   5. Creates and pushes the Go module git tag       (go/vX.Y.Z)
#
# Usage (from repo root):
#   bash post_commit.sh

set -e

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"

# ── Helpers ───────────────────────────────────────────────────────────────────

ask() {
  local prompt="$1"
  local var_name="$2"
  while true; do
    printf "%s: " "$prompt"
    read -r value
    if [ -z "$value" ]; then
      echo "  ⚠  Value cannot be empty. Please try again."
    elif ! echo "$value" | grep -qE '^v[0-9]+\.[0-9]+\.[0-9]+$'; then
      echo "  ⚠  Invalid format. Expected vX.Y.Z with a lowercase 'v' (e.g. v0.2.0)."
    else
      eval "$var_name=\"$value\""
      break
    fi
  done
}

ask_step() {
  local prompt="$1"
  local var_name="$2"
  printf "  %s [Y/n]: " "$prompt"
  read -r choice
  case "$choice" in
    n|N) eval "$var_name=false" ;;
    *)   eval "$var_name=true"  ;;
  esac
}

banner() {
  echo ""
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  $1"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo ""
}

check_npm_auth() {
  if npm whoami >/dev/null 2>&1; then
    echo "  ✅  Logged in to npm as: $(npm whoami)"
    return
  fi

  echo ""
  echo "  ⚠  Not logged in to npm."
  echo "  To authenticate, enter an npm access token."
  echo "  Generate one at: https://www.npmjs.com/settings/~/tokens"
  echo "  (Create an 'Automation' or 'Publish' token)"
  echo ""
  printf "  npm access token: "
  read -r NPM_TOKEN

  if [ -z "$NPM_TOKEN" ]; then
    echo "  ❌  No token provided. Aborting."
    exit 1
  fi

  # Write the token to ~/.npmrc (creates or updates the entry)
  NPM_RC="$HOME/.npmrc"
  # Remove any existing token line first to avoid duplicates
  if [ -f "$NPM_RC" ]; then
    grep -v '//registry.npmjs.org/:_authToken' "$NPM_RC" > "${NPM_RC}.tmp" && mv "${NPM_RC}.tmp" "$NPM_RC"
  fi
  echo "//registry.npmjs.org/:_authToken=${NPM_TOKEN}" >> "$NPM_RC"

  # Verify the token works
  if ! npm whoami >/dev/null 2>&1; then
    echo "  ❌  Token is invalid or has insufficient permissions."
    exit 1
  fi

  echo "  ✅  Logged in to npm as: $(npm whoami) (token saved to ~/.npmrc)"
}

# ── Collect versions ──────────────────────────────────────────────────────────

echo ""
echo "╔══════════════════════════════════════════════════╗"
echo "║          Finalytics Post-Commit Release          ║"
echo "╚══════════════════════════════════════════════════╝"
echo ""
echo "Please provide the version numbers for each release."
echo "FFI and Go/Python versions may differ."
echo ""

ask "FFI release version    (e.g. v0.2.0)" FFI_VERSION
ask "Python package version (e.g. v0.9.0)" PYTHON_VERSION
ask "Go module version      (e.g. v0.2.0)" GO_VERSION

PYTHON_TAG="python/${PYTHON_VERSION}"
GO_TAG="go/${GO_VERSION}"

# ── Select steps ──────────────────────────────────────────────────────────────

echo ""
echo "Which steps would you like to run?"
echo ""
ask_step "1. Cargo publish  — crates.io          (version from rust/Cargo.toml)" RUN_CARGO
ask_step "2. NPM publish    — npmjs.com           (version from js/package.json)" RUN_NPM
ask_step "3. FFI release    — GitHub ($FFI_VERSION)" RUN_FFI
ask_step "4. Python git tag — $PYTHON_TAG" RUN_PYTHON
ask_step "5. Go git tag     — $GO_TAG" RUN_GO

# ── Summary & confirm ─────────────────────────────────────────────────────────

echo ""
echo "Steps selected:"
[ "$RUN_CARGO"  = "true" ] && echo "  ✔  Cargo publish  (crates.io)"
[ "$RUN_NPM"    = "true" ] && echo "  ✔  NPM publish    (npmjs.com)"
[ "$RUN_FFI"    = "true" ] && echo "  ✔  FFI release    ($FFI_VERSION)"
[ "$RUN_PYTHON" = "true" ] && echo "  ✔  Python git tag ($PYTHON_TAG)"
[ "$RUN_GO"     = "true" ] && echo "  ✔  Go git tag     ($GO_TAG)"
echo ""
printf "Proceed? [y/N] "
read -r confirm
if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
  echo "Aborted."
  exit 0
fi

# ── Pre-flight checks ─────────────────────────────────────────────────────────

if [ "$RUN_NPM" = "true" ]; then
  banner "🔑  Checking npm auth"
  check_npm_auth
fi

# ── 1. Cargo Publish ──────────────────────────────────────────────────────────

if [ "$RUN_CARGO" = "true" ]; then
  banner "🦀  Cargo Publish — crates.io"
  cd "$REPO_ROOT/rust"
  cargo publish
  echo "✅  Rust crate published to crates.io"
fi

# ── 2. NPM Publish ────────────────────────────────────────────────────────────

if [ "$RUN_NPM" = "true" ]; then
  banner "🟨  NPM Publish — npmjs.com"
  cd "$REPO_ROOT/js"
  npm publish
  echo "✅  JS package published to npm"
fi

# ── 3. FFI Release ────────────────────────────────────────────────────────────

if [ "$RUN_FFI" = "true" ]; then
  banner "🔧  FFI Release — ffi/release_ffi.sh $FFI_VERSION"
  cd "$REPO_ROOT/ffi"
  bash release_ffi.sh "$FFI_VERSION"
fi

# ── 4. Python Git Tag ─────────────────────────────────────────────────────────

if [ "$RUN_PYTHON" = "true" ]; then
  banner "🐍  Python Git Tag — $PYTHON_TAG"
  cd "$REPO_ROOT"
  git tag "$PYTHON_TAG"
  git push origin "$PYTHON_TAG"
  echo "✅  Pushed tag: $PYTHON_TAG"
fi

# ── 5. Go Git Tag ─────────────────────────────────────────────────────────────

if [ "$RUN_GO" = "true" ]; then
  banner "🐹  Go Git Tag — $GO_TAG"
  cd "$REPO_ROOT"
  git tag "$GO_TAG"
  git push origin "$GO_TAG"
  echo "✅  Pushed tag: $GO_TAG"
fi

# ── Done ──────────────────────────────────────────────────────────────────────

echo ""
echo "🎉  All done!"
[ "$RUN_CARGO"  = "true" ] && echo "    Rust crate  : published to crates.io"
[ "$RUN_NPM"    = "true" ] && echo "    JS package  : published to npm"
[ "$RUN_FFI"    = "true" ] && echo "    FFI release : $FFI_VERSION"
[ "$RUN_PYTHON" = "true" ] && echo "    Python tag  : $PYTHON_TAG"
[ "$RUN_GO"     = "true" ] && echo "    Go tag      : $GO_TAG"
echo ""
