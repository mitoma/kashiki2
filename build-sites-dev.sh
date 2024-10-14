#!/bin/sh
GIT_ROOT_DIR=$(git rev-parse --show-toplevel)

cd "${GIT_ROOT_DIR}/sample_codes/showcase"
rm -rf "${GIT_ROOT_DIR}/site/src/wasm/showcase"
wasm-pack build --target web --out-dir "${GIT_ROOT_DIR}/site/src/wasm/showcase"

cd "${GIT_ROOT_DIR}/site"
mdbook clean
mdbook serve

cd "${GIT_ROOT_DIR}"
