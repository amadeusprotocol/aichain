#!/bin/bash
set -e

if ! command -v worker-build &> /dev/null; then
    echo "Installing worker-build..."
    cargo install worker-build
fi

echo "Building WASM..."
worker-build --release

rm -f build/.gitignore

echo "Build complete. Commit with: git add build/ && git commit -m 'Build WASM'"
echo "Build ocmplete. Next do git commit and do wrangler deploy"
