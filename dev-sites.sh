#!/bin/sh
GIT_ROOT_DIR=$(git rev-parse --show-toplevel)
OUT_DIR="${GIT_ROOT_DIR}/site/src/wasm/showcase"

cd "${GIT_ROOT_DIR}/sample_codes/showcase"
rm -rf "${OUT_DIR}"
echo "cargo build"
cargo build --target wasm32-unknown-unknown --release
echo "wasm-bindgen"
wasm-bindgen "${GIT_ROOT_DIR}/target/wasm32-unknown-unknown/release/showcase.wasm" --target web --out-dir "${OUT_DIR}"
echo "wasm-opt"
wasm-opt -Oz -o "${OUT_DIR}/showcase_bg.wasm.out" "${OUT_DIR}/showcase_bg.wasm"
rm "${OUT_DIR}/showcase_bg.wasm"
mv "${OUT_DIR}/showcase_bg.wasm.out" "${OUT_DIR}/showcase_bg.wasm"

cd "${GIT_ROOT_DIR}/site"
mdbook clean
mdbook serve

cd "${GIT_ROOT_DIR}"
