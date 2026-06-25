#!/usr/bin/env bash
set -euo pipefail

# This script is executed from the project root directory (because of cd ../.. in tauri.conf.json beforeBuildCommand)
ROOT_DIR="$(pwd)"
SRC_TAURI_DIR="${ROOT_DIR}/desktop/tauri"

TARGET_TRIPLE="${TARGET_TRIPLE:-$(rustc -vV 2>/dev/null | grep host | cut -d' ' -f2)}"

# Tauri sidecar naming convention: binaries/<name>-<target-triple>(.exe)
SIDECAR_NAME="nova-${TARGET_TRIPLE}"
if [[ "${TARGET_TRIPLE}" == *"windows"* ]]; then
  SIDECAR_NAME="${SIDECAR_NAME}.exe"
fi

echo "==> Tauri pre-build"
echo "    ROOT_DIR: ${ROOT_DIR}"
echo "    TARGET_TRIPLE: ${TARGET_TRIPLE}"
echo "    SIDECAR: ${SIDECAR_NAME}"
echo "    SRC_TAURI_DIR: ${SRC_TAURI_DIR}"

mkdir -p "${SRC_TAURI_DIR}/binaries"

GOOS=""
GOARCH=""

case "${TARGET_TRIPLE}" in
  x86_64-apple-darwin)
    GOOS="darwin"; GOARCH="amd64"
    ;;
  aarch64-apple-darwin)
    GOOS="darwin"; GOARCH="arm64"
    ;;
  x86_64-pc-windows-msvc)
    GOOS="windows"; GOARCH="amd64"
    ;;
  x86_64-unknown-linux-gnu)
    GOOS="linux"; GOARCH="amd64"
    ;;
  aarch64-unknown-linux-gnu)
    GOOS="linux"; GOARCH="arm64"
    ;;
  *)
    echo "Warning: unknown TARGET_TRIPLE '${TARGET_TRIPLE}', native build"
    UNAME_S="$(uname -s)"
    UNAME_M="$(uname -m)"
    case "${UNAME_S}" in
      Darwin) GOOS="darwin" ;;
      Linux) GOOS="linux" ;;
      MINGW*|MSYS*|CYGWIN*) GOOS="windows" ;;
      *) GOOS="linux" ;;
    esac
    case "${UNAME_M}" in
      x86_64|amd64) GOARCH="amd64" ;;
      arm64|aarch64) GOARCH="arm64" ;;
      *) GOARCH="amd64" ;;
    esac
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
OUTPUT_BINARY="${SRC_TAURI_DIR}/binaries/${SIDECAR_NAME}"
CGO_ENABLED=0 GOOS="${GOOS}" GOARCH="${GOARCH}" go build -trimpath \
  -ldflags "-s -w -X nova/internal/buildinfo.Version=${VERSION}" \
  -o "${OUTPUT_BINARY}" \
  ./cmd/nova/

if [ "${GOOS}" != "windows" ]; then
  chmod +x "${OUTPUT_BINARY}"
fi

echo "==> Copying resources..."
rm -rf "${SRC_TAURI_DIR}/binaries/web" "${SRC_TAURI_DIR}/binaries/skills"
cp -R "${ROOT_DIR}/skills" "${SRC_TAURI_DIR}/binaries/"
if [ -f "${ROOT_DIR}/config.toml" ]; then
  cp "${ROOT_DIR}/config.toml" "${SRC_TAURI_DIR}/binaries/"
fi
mkdir -p "${SRC_TAURI_DIR}/binaries/web"
cp -R "${ROOT_DIR}/web/dist/"* "${SRC_TAURI_DIR}/binaries/web/"

echo "==> Pre-build complete"
ls -la "${SRC_TAURI_DIR}/binaries/"
