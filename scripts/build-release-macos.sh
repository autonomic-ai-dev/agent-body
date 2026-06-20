#!/usr/bin/env bash
# Build release binaries for macOS (x86_64 + aarch64)
set -euo pipefail

NAME="agent-body"
VERSION="${1:-$(git describe --tags --always 2>/dev/null || echo "0.1.0")}"

echo "Building $NAME $VERSION for macOS..."

cargo build --release --target x86_64-apple-darwin -p "$NAME"
cargo build --release --target aarch64-apple-darwin -p "$NAME"

mkdir -p dist
cp "target/x86_64-apple-darwin/release/$NAME" "dist/${NAME}-x86_64-apple-darwin"
cp "target/aarch64-apple-darwin/release/$NAME" "dist/${NAME}-aarch64-apple-darwin"

echo "Done: dist/"
ls -lh dist/
