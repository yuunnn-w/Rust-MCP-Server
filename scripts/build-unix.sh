#!/bin/bash
set -e

# Rust MCP Server Build Script for Unix/Linux/macOS
# https://github.com/yuunnn-w/Rust-MCP-Server

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Rust MCP Server Build Script (Unix)   ${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${PROJECT_ROOT}"

# Check required tools
check_command() {
    if ! command -v $1 &> /dev/null; then
        echo -e "${RED}Error: $1 not found, please install $1${NC}"
        exit 1
    fi
}

echo "Checking build environment..."
check_command rustc
check_command cargo

# Check for download tool
if command -v curl &> /dev/null; then
    DOWNLOAD_TOOL="curl"
elif command -v wget &> /dev/null; then
    DOWNLOAD_TOOL="wget"
else
    echo -e "${YELLOW}Warning: Neither curl nor wget found. Will skip downloading Chart.js.${NC}"
    DOWNLOAD_TOOL="none"
fi

echo -e "${GREEN}Build environment OK${NC}"
echo ""

# Create static directory if it doesn't exist
mkdir -p src/web/static

# Download Chart.js if not exists
if [ "$DOWNLOAD_TOOL" != "none" ]; then
    echo "Downloading frontend dependencies..."
    cd src/web/static
    
    if [ ! -f "chart.min.js" ]; then
        echo "Downloading Chart.js..."
        if [ "$DOWNLOAD_TOOL" = "wget" ]; then
            wget -q --show-progress https://cdn.jsdelivr.net/npm/chart.js@4.5.1/dist/chart.umd.min.js -O chart.min.js 2>/dev/null || wget -q https://cdn.jsdelivr.net/npm/chart.js@4.5.1/dist/chart.umd.min.js -O chart.min.js
        else
            curl -# -L https://cdn.jsdelivr.net/npm/chart.js@4.5.1/dist/chart.umd.min.js -o chart.min.js
        fi
        echo -e "${GREEN}Chart.js downloaded successfully${NC}"
    else
        echo "Chart.js already exists, skipping download"
    fi
    
    cd ../../..
fi

echo ""
echo -e "${BLUE}Building Rust MCP Server...${NC}"
echo ""

# Build the main project
export RUSTFLAGS="-C target-feature=+crt-static"
cargo build --release

# Check if build succeeded
if [ ! -f "target/release/rust-mcp-server" ]; then
    echo -e "${RED}Build failed: rust-mcp-server executable not found${NC}"
    exit 1
fi

# Copy main executable to project root
cp target/release/rust-mcp-server ./rust-mcp-server
echo -e "${GREEN}Main server executable copied: ./rust-mcp-server${NC}"

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}       Build completed successfully!      ${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Quick Start:"
echo "  ./rust-mcp-server                    # Start the MCP server with default settings"
echo "  ./rust-mcp-server --webui-port 8080  # Start with custom WebUI port"
echo ""
echo "Testing with llama.cpp:"
echo "  # Start this server first"
echo "  ./rust-mcp-server --mcp-transport http --mcp-port 8080"
echo ""
echo "  # Then start llama-server with MCP config:"
echo "  llama-server -m your-model.gguf --mcp-config-url http://localhost:8080/config"
echo ""
echo "Help:"
echo "  ./rust-mcp-server --help             # Show all available options"
echo ""
echo "Documentation:"
echo "  README.md                            # English documentation"
echo "  README-zh.md                         # Chinese documentation"
echo "  docs/                                # Detailed documentation"
echo ""
