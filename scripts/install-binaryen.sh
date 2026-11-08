#!/usr/bin/env bash
# Installs wasm-opt from Binaryen's release at the version the npm package is built with,
# checking the download against its pinned SHA-256 digest.
#
#   scripts/install-binaryen.sh            installs into ~/.local/binaryen
#   scripts/install-binaryen.sh DIRECTORY  installs into DIRECTORY
#
# Then put DIRECTORY/bin on PATH. bindings/wasm/build.sh reads the version from this file and
# refuses to build with any other.
set -euo pipefail

version=123

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) platform=x86_64-linux sha256=e959f2170af4c20c552e9de3a0253704d6a9d2766e8fdb88e4d6ac4bae9388fe ;;
  Darwin-arm64) platform=arm64-macos sha256=74428be348c1a09863e7b642a1fa948cabf8ec9561052233d8288e941951725b ;;
  Darwin-x86_64) platform=x86_64-macos sha256=cc18b14d2b673d9c66bf54f31ff2b0ceb23ba5132455b893965ae2792f9e00dd ;;
  *)
    echo "no Binaryen $version release for $(uname -s) $(uname -m); build it from" \
      "https://github.com/WebAssembly/binaryen/tree/version_$version" >&2
    exit 1
    ;;
esac

target=${1:-$HOME/.local/binaryen}
archive=binaryen-version_$version-$platform.tar.gz
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

curl --fail --silent --show-error --location --output "$work/$archive" \
  "https://github.com/WebAssembly/binaryen/releases/download/version_$version/$archive"
if command -v sha256sum >/dev/null; then
  found=$(sha256sum "$work/$archive" | cut -d' ' -f1)
else
  found=$(shasum -a 256 "$work/$archive" | cut -d' ' -f1)
fi
if [ "$found" != "$sha256" ]; then
  echo "$archive has SHA-256 $found, not the pinned $sha256" >&2
  exit 1
fi

# wasm-opt, and the library it is linked with on macOS.
tar -xzf "$work/$archive" -C "$work" "binaryen-version_$version/bin/wasm-opt" "binaryen-version_$version/lib"
mkdir -p "$target"
rm -rf "$target/bin" "$target/lib"
mv "$work/binaryen-version_$version/bin" "$work/binaryen-version_$version/lib" "$target/"
"$target/bin/wasm-opt" --version
echo "Installed wasm-opt into $target/bin; add it to PATH."
