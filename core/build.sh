#!/bin/bash
set -e
cd "$(dirname $0)"
source ./flags.sh
cargo build --all --target wasm32-unknown-unknown --release
mkdir res || true
cp target/wasm32-unknown-unknown/release/*.wasm ./res/
(cd ../multi-token-standard-impl/examples/multi-token && ./build.sh && cp res/multi_token.wasm ../../../core/res)
