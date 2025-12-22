#!/usr/bin/env bash

set -euo pipefail

STATIC_DIR=${STATIC_DIR:-"./static"}

# Download `dist.zip` from the latest release of qdrant-web-ui
# vectX is Qdrant API compatible, so we can use the Qdrant Web UI

DOWNLOAD_LINK="https://github.com/qdrant/qdrant-web-ui/releases/latest/download/dist-qdrant.zip"

echo "Downloading Qdrant Web UI..."

if command -v wget &> /dev/null
then
    wget -q -O dist-qdrant.zip $DOWNLOAD_LINK
else
    curl -sL -o dist-qdrant.zip $DOWNLOAD_LINK
fi

# Clean and extract
mkdir -p "${STATIC_DIR}"
rm -rf "${STATIC_DIR}/"* 2>/dev/null || true
unzip -q -o dist-qdrant.zip -d "${STATIC_DIR}"
rm dist-qdrant.zip

# Move files from dist subfolder to static root
if [ -d "${STATIC_DIR}/dist" ]; then
    cp -r "${STATIC_DIR}/dist/"* "${STATIC_DIR}"
    rm -rf "${STATIC_DIR}/dist"
fi

echo "Web UI synced to ${STATIC_DIR}"
