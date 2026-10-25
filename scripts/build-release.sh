#!/usr/bin/env bash
# Native Linux/macOS packaging; run from any working directory.
set -euo pipefail
cd "$(dirname "$0")/.."
target="${1:-$(rustc -vV | sed -n 's/^host: //p')}"
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
artifact="sheetdiff-v${version}-${target}"
cargo build --locked --release --target "$target"
mkdir -p "dist/$artifact"
cp "target/$target/release/sheetdiff" README.md LICENSE "dist/$artifact/"
tar -czf "dist/$artifact.tar.gz" -C dist "$artifact"
(
  cd dist
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$artifact.tar.gz" > "$artifact.tar.gz.sha256"
  else
    shasum -a 256 "$artifact.tar.gz" > "$artifact.tar.gz.sha256"
  fi
)
printf 'Archive: dist/%s.tar.gz\n' "$artifact"
