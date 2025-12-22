#!/bin/bash
# Script to yank old distx packages and publish new vectx packages
# 
# Usage:
#   ./scripts/publish_vectx.sh [--yank-only] [--publish-only]
#
# Options:
#   --yank-only    Only yank distx packages, don't publish
#   --publish-only Only publish vectx packages, don't yank

set -e

YANK_ONLY=false
PUBLISH_ONLY=false

# Parse arguments
for arg in "$@"; do
    case $arg in
        --yank-only)
            YANK_ONLY=true
            shift
            ;;
        --publish-only)
            PUBLISH_ONLY=true
            shift
            ;;
        *)
            ;;
    esac
done

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================"
echo "vectX Package Migration Script"
echo "========================================"
echo ""

# Step 1: Yank old distx packages
if [ "$PUBLISH_ONLY" = false ]; then
    echo -e "${YELLOW}Step 1: Yanking old distx packages${NC}"
    echo ""
    
    # List of potential distx package names
    DISTX_PACKAGES=("distx" "distx-core" "distx-storage" "distx-api")
    
    echo "Note: You need to yank each version individually."
    echo "To see all versions of a package, visit: https://crates.io/crates/<package-name>/versions"
    echo ""
    
    for package in "${DISTX_PACKAGES[@]}"; do
        echo -e "${YELLOW}Checking if ${package} exists on crates.io...${NC}"
        
        # Check if package exists (this will fail if it doesn't exist)
        if cargo search --limit 1 "$package" 2>/dev/null | grep -q "^${package}"; then
            echo -e "${RED}Package ${package} found on crates.io${NC}"
            echo "To yank all versions, you'll need to:"
            echo "  1. Visit https://crates.io/crates/${package}/versions"
            echo "  2. For each version, run: cargo yank --vers <version> ${package}"
            echo ""
            echo "Or use this script interactively:"
            read -p "Do you want to yank all versions of ${package}? (y/n) " -n 1 -r
            echo
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                echo "Enter version numbers to yank (one per line, empty line to finish):"
                while IFS= read -r version; do
                    [ -z "$version" ] && break
                    echo -e "${YELLOW}Yanking ${package} version ${version}...${NC}"
                    cargo yank --vers "$version" "$package" || echo -e "${RED}Failed to yank ${package} ${version}${NC}"
                done
            fi
        else
            echo -e "${GREEN}Package ${package} not found on crates.io (or doesn't exist)${NC}"
        fi
        echo ""
    done
    
    echo -e "${GREEN}Yanking step complete!${NC}"
    echo ""
fi

# Step 2: Publish new vectx packages
if [ "$YANK_ONLY" = false ]; then
    echo -e "${YELLOW}Step 2: Publishing new vectx packages${NC}"
    echo ""
    
    # Verify we're in the right directory
    if [ ! -f "Cargo.toml" ]; then
        echo -e "${RED}Error: Cargo.toml not found. Please run this script from the distx root directory.${NC}"
        exit 1
    fi
    
    # Check if logged in to crates.io (check for credentials file)
    CREDENTIALS_FILE="${CARGO_HOME:-$HOME/.cargo}/credentials.toml"
    if [ ! -f "$CREDENTIALS_FILE" ]; then
        echo -e "${YELLOW}You need to login to crates.io first.${NC}"
        echo "Get your API token from: https://crates.io/me"
        echo "Then run: cargo login <your-token>"
        exit 1
    fi
    
    # Verify package names
    echo "Verifying package names..."
    MAIN_NAME=$(grep '^name =' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    if [ "$MAIN_NAME" != "vectx" ]; then
        echo -e "${RED}Error: Main package name is '${MAIN_NAME}', expected 'vectx'${NC}"
        exit 1
    fi
    echo -e "${GREEN}Main package: ${MAIN_NAME}${NC}"
    echo ""
    
    # Publish in dependency order
    echo "Publishing packages in dependency order..."
    echo ""
    
    # 1. vectx-core (no dependencies on other workspace members)
    echo -e "${YELLOW}[1/4] Publishing vectx-core...${NC}"
    cd lib/core
    cargo publish --dry-run
    read -p "Publish vectx-core? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cargo publish
        echo -e "${GREEN}✓ vectx-core published${NC}"
    else
        echo -e "${YELLOW}Skipped vectx-core${NC}"
    fi
    cd ../..
    echo ""
    
    # 2. vectx-storage (depends on vectx-core)
    echo -e "${YELLOW}[2/4] Publishing vectx-storage...${NC}"
    cd lib/storage
    cargo publish --dry-run
    read -p "Publish vectx-storage? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cargo publish
        echo -e "${GREEN}✓ vectx-storage published${NC}"
    else
        echo -e "${YELLOW}Skipped vectx-storage${NC}"
    fi
    cd ../..
    echo ""
    
    # 3. vectx-api (depends on vectx-core and vectx-storage)
    echo -e "${YELLOW}[3/4] Publishing vectx-api...${NC}"
    cd lib/api
    cargo publish --dry-run
    read -p "Publish vectx-api? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cargo publish
        echo -e "${GREEN}✓ vectx-api published${NC}"
    else
        echo -e "${YELLOW}Skipped vectx-api${NC}"
    fi
    cd ../..
    echo ""
    
    # 4. vectx (main package, depends on all above)
    echo -e "${YELLOW}[4/4] Publishing vectx (main package)...${NC}"
    cargo publish --dry-run
    read -p "Publish vectx? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cargo publish
        echo -e "${GREEN}✓ vectx published${NC}"
    else
        echo -e "${YELLOW}Skipped vectx${NC}"
    fi
    echo ""
    
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
fi

