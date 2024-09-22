#!/bin/bash
set -eux

TAG="$1"
REPO_ROOT=$(git rev-parse --show-toplevel)
VERSION=$(sver calc .)
ARTIFACT_DIR=$(mktemp -d)
trap 'rm -rf "$ARTIFACT_DIR"' EXIT

TARGET_OS_ARCHES=(
  "x86_64-unknown-linux-gnu linux amd64"
  "x86_64-pc-windows-gnu windows amd64"
  "aarch64-apple-darwin macos arm64"
)

for TARGET_OS_ARCH in "${TARGET_OS_ARCHES[@]}"; do
  read -r TARGET OS ARCH <<<"$TARGET_OS_ARCH"
  cd "$REPO_ROOT"
  DIR=$(mktemp -d)
  gh run download --name "kashikishi-${TARGET}-${VERSION}" --dir "$DIR"
  cd "$DIR"
  if [[ "$OS" != windows ]]; then
    chmod +x kashikishi
  fi
  zip "${ARTIFACT_DIR}/kashikishi_${TAG}_${OS}_${ARCH}.zip" ./*
  rm -rf "$DIR"
done

cd "${ARTIFACT_DIR}"
sha256sum * > SHASUMS256.txt

cd "$REPO_ROOT"
gh release create "$TAG" --generate-notes --draft
gh release upload "$TAG" "$ARTIFACT_DIR"/*
