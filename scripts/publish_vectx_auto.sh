#!/bin/bash
# Automated script to publish all vectx packages to crates.io
# 
# Usage:
#   ./scripts/publish_vectx_auto.sh [--dry-run]
#
# Options:
#   --dry-run    Run cargo publish --dry-run for all packages without publishing

set -e

DRY_RUN=false

if [[ "$1" == "--dry-run" ]]; then
    DRY_RUN=true
    echo "DRY RUN MODE - No packages will be published"
fi

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "========================================"
echo "Publishing vectx packages to crates.io"
echo "========================================"
echo ""

# Verify we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Cargo.toml not found. Please run this script from the distx root directory.${NC}"
    exit 1
fi

# Check if logged in to crates.io (check for credentials file)
CREDENTIALS_FILE="${CARGO_HOME:-$HOME/.cargo}/credentials.toml"
if [ ! -f "$CREDENTIALS_FILE" ]; then
    echo -e "${RED}Error: You need to login to crates.io first.${NC}"
    echo "Get your API token from: https://crates.io/me"
    echo "Then run: cargo login <your-token>"
    exit 1
fi

# Verify package names
MAIN_NAME=$(grep '^name =' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
if [ "$MAIN_NAME" != "vectx" ]; then
    echo -e "${RED}Error: Main package name is '${MAIN_NAME}', expected 'vectx'${NC}"
    exit 1
fi

echo -e "${GREEN}Main package: ${MAIN_NAME}${NC}"
echo ""

# Check for uncommitted changes and set ALLOW_DIRTY flag if needed
ALLOW_DIRTY=""
if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
    echo -e "${YELLOW}Warning: Uncommitted changes detected in git working directory${NC}"
    echo "Checking if changes are in excluded directories..."
    
    # Check if all changes are in examples or other excluded directories
    UNCOMMITTED=$(git status --porcelain 2>/dev/null | grep -v "^??" || true)
    if [ -n "$UNCOMMITTED" ]; then
        # Check if changes are only in examples, data, scripts, or tests directories
        EXCLUDED_PATHS=$(echo "$UNCOMMITTED" | grep -E "^\s*[MADRC]\s+(examples/|data/|scripts/|tests/)" || true)
        ALL_COUNT=$(echo "$UNCOMMITTED" | wc -l | tr -d ' ')
        EXCLUDED_COUNT=$(echo "$EXCLUDED_PATHS" | grep -c . || echo "0")
        
        if [ "$ALL_COUNT" = "$EXCLUDED_COUNT" ] && [ "$EXCLUDED_COUNT" -gt 0 ]; then
            echo -e "${YELLOW}All uncommitted changes are in excluded directories (examples/data/scripts/tests)${NC}"
            echo -e "${YELLOW}Using --allow-dirty flag for cargo publish${NC}"
            ALLOW_DIRTY="--allow-dirty"
        else
            echo -e "${RED}Error: Uncommitted changes detected in non-excluded directories${NC}"
            echo "Please commit or stash your changes before publishing:"
            git status --short 2>/dev/null | head -10
            echo ""
            echo "Or use --allow-dirty flag manually if you're sure:"
            echo "  cargo publish --allow-dirty"
            exit 1
        fi
    fi
    echo ""
fi

# Publish in dependency order
echo "Publishing packages in dependency order..."
echo ""

# 1. vectx-core
echo -e "${YELLOW}[1/4] Publishing vectx-core...${NC}"
cd lib/core
if [ "$DRY_RUN" = true ]; then
    cargo publish --dry-run $ALLOW_DIRTY
else
    cargo publish $ALLOW_DIRTY
    echo -e "${GREEN}✓ vectx-core published${NC}"
fi
cd ../..
echo ""

# Wait a bit for crates.io to index (optional but recommended)
if [ "$DRY_RUN" = false ]; then
    echo "Waiting 10 seconds for crates.io to index..."
    sleep 10
fi

# 2. vectx-storage
echo -e "${YELLOW}[2/4] Publishing vectx-storage...${NC}"
cd lib/storage
if [ "$DRY_RUN" = true ]; then
    cargo publish --dry-run $ALLOW_DIRTY
else
    cargo publish $ALLOW_DIRTY
    echo -e "${GREEN}✓ vectx-storage published${NC}"
fi
cd ../..
echo ""

if [ "$DRY_RUN" = false ]; then
    echo "Waiting 10 seconds for crates.io to index..."
    sleep 10
fi

# 3. vectx-api
echo -e "${YELLOW}[3/4] Publishing vectx-api...${NC}"
cd lib/api
if [ "$DRY_RUN" = true ]; then
    cargo publish --dry-run $ALLOW_DIRTY
else
    cargo publish $ALLOW_DIRTY
    echo -e "${GREEN}✓ vectx-api published${NC}"
fi
cd ../..
echo ""

if [ "$DRY_RUN" = false ]; then
    echo "Waiting 10 seconds for crates.io to index..."
    sleep 10
fi

# 4. vectx (main package)
echo -e "${YELLOW}[4/4] Publishing vectx (main package)...${NC}"
if [ "$DRY_RUN" = true ]; then
    cargo publish --dry-run $ALLOW_DIRTY
else
    cargo publish $ALLOW_DIRTY
    echo -e "${GREEN}✓ vectx published${NC}"
fi
echo ""

if [ "$DRY_RUN" = false ]; then
    echo "========================================"
    echo -e "${GREEN}All packages published successfully!${NC}"
    echo "========================================"
    echo ""
    echo "Your packages are now available at:"
    echo "  - https://crates.io/crates/vectx"
    echo "  - https://crates.io/crates/vectx-core"
    echo "  - https://crates.io/crates/vectx-storage"
    echo "  - https://crates.io/crates/vectx-api"
    echo ""
else
    echo "========================================"
    echo -e "${YELLOW}Dry run complete! Run without --dry-run to actually publish.${NC}"
    echo "========================================"
fi

