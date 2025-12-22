#!/bin/bash
# vectX Release Script
# Creates release builds for the current platform

set -e

VERSION="${1:-$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')}"
RELEASE_DIR="releases/v${VERSION}"
BINARY_NAME="vectx"

echo "========================================"
echo "vectX Release Builder v${VERSION}"
echo "========================================"

# Create release directory
mkdir -p "${RELEASE_DIR}"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "${OS}" in
    darwin) OS_NAME="apple-darwin" ;;
    linux) OS_NAME="unknown-linux-gnu" ;;
    *) OS_NAME="${OS}" ;;
esac

case "${ARCH}" in
    x86_64) ARCH_NAME="x86_64" ;;
    arm64|aarch64) ARCH_NAME="aarch64" ;;
    *) ARCH_NAME="${ARCH}" ;;
esac

TARGET="${ARCH_NAME}-${OS_NAME}"
RELEASE_NAME="${BINARY_NAME}-${TARGET}"

echo "Building for target: ${TARGET}"
echo ""

# Build release
echo "Building release binary..."
cargo build --release

# Create release package
echo "Creating release package..."
TEMP_DIR=$(mktemp -d)
mkdir -p "${TEMP_DIR}/${BINARY_NAME}"

# Copy binary
cp "target/release/${BINARY_NAME}" "${TEMP_DIR}/${BINARY_NAME}/"

# Copy documentation
cp README.md "${TEMP_DIR}/${BINARY_NAME}/" 2>/dev/null || true
cp LICENSE-MIT "${TEMP_DIR}/${BINARY_NAME}/" 2>/dev/null || true
cp LICENSE-APACHE "${TEMP_DIR}/${BINARY_NAME}/" 2>/dev/null || true

# Create archive
cd "${TEMP_DIR}"
if [[ "${OS}" == "darwin" || "${OS}" == "linux" ]]; then
    tar -czvf "${RELEASE_NAME}.tar.gz" "${BINARY_NAME}"
    ARCHIVE="${RELEASE_NAME}.tar.gz"
else
    zip -r "${RELEASE_NAME}.zip" "${BINARY_NAME}"
    ARCHIVE="${RELEASE_NAME}.zip"
fi

# Move to release directory
cd -
mv "${TEMP_DIR}/${ARCHIVE}" "${RELEASE_DIR}/"

# Calculate SHA256
cd "${RELEASE_DIR}"
SHA256=$(shasum -a 256 "${ARCHIVE}" | cut -d' ' -f1)
echo "${SHA256}  ${ARCHIVE}" > "${ARCHIVE}.sha256"

# Cleanup
rm -rf "${TEMP_DIR}"

echo ""
echo "========================================"
echo "Release built successfully!"
echo "========================================"
echo ""
echo "Archive: ${RELEASE_DIR}/${ARCHIVE}"
echo "SHA256:  ${SHA256}"
echo ""
echo "To create a GitHub release:"
echo "  git tag -a v${VERSION} -m 'Release v${VERSION}'"
echo "  git push origin v${VERSION}"
echo "  gh release create v${VERSION} ${RELEASE_DIR}/* --title 'v${VERSION}' --notes-file RELEASE_NOTES.md"
