#!/bin/sh
GIT_ROOT_DIR=$(git rev-parse --show-toplevel)

# ビルド関数を定義
build_wasm_project() {
    PROJECT_NAME=$1
    PROJECT_DIR=$2
    
    OUT_DIR="${GIT_ROOT_DIR}/site/src/wasm/${PROJECT_NAME}"
    
    cd "${PROJECT_DIR}"
    rm -rf "${OUT_DIR}"
    echo "Building ${PROJECT_NAME}..."
    echo "  cargo build"
    cargo build --target wasm32-unknown-unknown --release
    echo "  wasm-bindgen"
    wasm-bindgen "${GIT_ROOT_DIR}/target/wasm32-unknown-unknown/release/${PROJECT_NAME}.wasm" --target web --out-dir "${OUT_DIR}"
    echo "  wasm-opt"
    wasm-opt -Oz -o "${OUT_DIR}/${PROJECT_NAME}_bg.wasm.out" "${OUT_DIR}/${PROJECT_NAME}_bg.wasm"
    rm "${OUT_DIR}/${PROJECT_NAME}_bg.wasm"
    mv "${OUT_DIR}/${PROJECT_NAME}_bg.wasm.out" "${OUT_DIR}/${PROJECT_NAME}_bg.wasm"
    echo "  ${PROJECT_NAME} build complete"
}

# showcase をビルド
build_wasm_project "showcase" "${GIT_ROOT_DIR}/sample_codes/showcase"

# oogiri_gen をビルド
build_wasm_project "oogiri_gen" "${GIT_ROOT_DIR}/sample_codes/oogiri_gen"

cd "${GIT_ROOT_DIR}/site"
mdbook clean
mdbook serve

cd "${GIT_ROOT_DIR}"
