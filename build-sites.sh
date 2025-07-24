#!/bin/sh
GIT_ROOT_DIR=$(git rev-parse --show-toplevel)
OUT_DIR="${GIT_ROOT_DIR}/site/src/wasm/showcase"

cd "${GIT_ROOT_DIR}/sample_codes/showcase"
rm -rf "${OUT_DIR}"
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen "${GIT_ROOT_DIR}/target/wasm32-unknown-unknown/release/showcase.wasm" --target web --out-dir "${OUT_DIR}"
wasm-opt -Oz -o "${OUT_DIR}/showcase_bg.wasm.out" "${OUT_DIR}/showcase_bg.wasm"
rm "${OUT_DIR}/showcase_bg.wasm"
mv "${OUT_DIR}/showcase_bg.wasm.out" "${OUT_DIR}/showcase_bg.wasm"

cd "${GIT_ROOT_DIR}/site"
mdbook clean
mdbook build

cd "${GIT_ROOT_DIR}"
