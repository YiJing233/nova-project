#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARIES_DIR="${ROOT_DIR}/desktop/tauri/binaries"
TARGET_TRIPLE="${TARGET_TRIPLE:-$(rustc -vV 2>/dev/null | grep host | cut -d' ' -f2)}"

echo "==> Tauri pre-build: building Go binary for ${TARGET_TRIPLE}"
echo "    ROOT_DIR: ${ROOT_DIR}"
echo "    BINARIES_DIR: ${BINARIES_DIR}"

mkdir -p "${BINARIES_DIR}/${TARGET_TRIPLE}"

GOOS=""
GOARCH=""
EXE=""

case "${TARGET_TRIPLE}" in
  x86_64-apple-darwin)
    GOOS="darwin"; GOARCH="amd64"
    ;;
  aarch64-apple-darwin)
    GOOS="darwin"; GOARCH="arm64"
    ;;
  x86_64-pc-windows-msvc)
    GOOS="windows"; GOARCH="amd64"; EXE=".exe"
    ;;
  x86_64-unknown-linux-gnu)
    GOOS="linux"; GOARCH="amd64"
    ;;
  aarch64-unknown-linux-gnu)
    GOOS="linux"; GOARCH="arm64"
    ;;
  *)
    echo "Warning: unknown TARGET_TRIPLE '${TARGET_TRIPLE}', attempting native build"
    ;;
esac

VERSION="$(node -p "require('${ROOT_DIR}/web/package.json').version" 2>/dev/null || echo "dev")"

cd "${ROOT_DIR}"

echo "==> Building web frontend..."
if [ ! -d "web/node_modules" ]; then
  (cd web && pnpm install --frozen-lockfile)
fi
(cd web && pnpm build)

echo "==> Building Go binary GOOS=${GOOS} GOARCH=${GOARCH}"
BINARY_NAME="nova${EXE}"
CGO_ENABLED=0 go build -trimpath \
  -ldflags "-s -w -X nova/internal/buildinfo.Version=${VERSION}" \
  -o "${BINARIES_DIR}/${TARGET_TRIPLE}/${BINARY_NAME}" \
  ./cmd/nova/

if [ "${GOOS}" != "windows" ]; then
  chmod +x "${BINARIES_DIR}/${TARGET_TRIPLE}/${BINARY_NAME}"
fi

echo "==> Copying resources to binaries directory..."
cp -R "${ROOT_DIR}/skills" "${BINARIES_DIR}/${TARGET_TRIPLE}/skills"
if [ -f "${ROOT_DIR}/config.toml" ]; then
  cp "${ROOT_DIR}/config.toml" "${BINARIES_DIR}/${TARGET_TRIPLE}/config.toml"
fi
mkdir -p "${BINARIES_DIR}/${TARGET_TRIPLE}/web"
cp -R "${ROOT_DIR}/web/dist/"* "${BINARIES_DIR}/${TARGET_TRIPLE}/web/"

echo "==> Pre-build complete"
ls -la "${BINARIES_DIR}/${TARGET_TRIPLE}/"
