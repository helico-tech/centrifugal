#!/bin/sh
# Builds the browser version into dist/: index.html, the wasm and the JS
# loader that ships with the macroquad version in Cargo.lock.
set -eu
cd "$(dirname "$0")/.."
cargo build --release --target wasm32-unknown-unknown
MQ_DIR=$(cargo metadata --format-version 1 | python3 -c '
import json, os, sys
pkgs = json.load(sys.stdin)["packages"]
print(next(os.path.dirname(p["manifest_path"]) for p in pkgs if p["name"] == "macroquad"))')
rm -rf dist
mkdir -p dist
cp web/index.html "$MQ_DIR/js/mq_js_bundle.js" dist/
cp target/wasm32-unknown-unknown/release/centrifugal.wasm dist/
ls -l dist
