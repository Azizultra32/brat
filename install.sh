#!/bin/bash
set -e

# Brat Installer Script
# Usage: curl -fsSL https://raw.githubusercontent.com/YOUR_ORG/brat/main/install.sh | bash
#
# Environment variables:
#   BRAT_VERSION     - Version to install (default: latest)
#   BRAT_INSTALL_DIR - Installation directory (default: /usr/local/bin)

VERSION="${BRAT_VERSION:-latest}"
INSTALL_DIR="${BRAT_INSTALL_DIR:-/usr/local/bin}"
REPO="${BRAT_REPO:-YOUR_ORG/brat}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }
debug() { [ "${BRAT_DEBUG:-}" = "1" ] && echo -e "${BLUE}[DEBUG]${NC} $1"; }

# Detect OS and architecture
detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="macos" ;;
        MINGW*|MSYS*|CYGWIN*) os="windows" ;;
        *) error "Unsupported operating system: $(uname -s)" ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *) error "Unsupported architecture: $(uname -m)" ;;
    esac

    echo "${os}-${arch}"
}

# Get latest version from GitHub API
get_latest_version() {
    local latest
    latest=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/')
    if [ -z "$latest" ]; then
        error "Failed to fetch latest version from GitHub"
    fi
    echo "$latest"
}

# Check for required commands
check_deps() {
    if ! command -v curl &> /dev/null; then
        error "curl is required but not installed. Please install curl and try again."
    fi

    if ! command -v tar &> /dev/null && [[ "$(uname -s)" != MINGW* ]]; then
        error "tar is required but not installed. Please install tar and try again."
    fi
}

# Download and install
install() {
    local platform version url artifact temp_dir binary_name

    platform=$(detect_platform)
    info "Detected platform: $platform"

    if [ "$VERSION" = "latest" ]; then
        info "Fetching latest version..."
        version=$(get_latest_version)
    else
        version="$VERSION"
    fi
    info "Installing version: $version"

    # Determine artifact name and binary extension
    if [[ "$platform" == *"windows"* ]]; then
        artifact="brat-${platform}.zip"
        binary_name="brat.exe"
    else
        artifact="brat-${platform}.tar.gz"
        binary_name="brat"
    fi

    url="https://github.com/${REPO}/releases/download/v${version}/${artifact}"
    info "Downloading from: $url"

    # Create temp directory
    temp_dir=$(mktemp -d)
    trap "rm -rf $temp_dir" EXIT

    # Download archive
    if ! curl -fsSL "$url" -o "${temp_dir}/${artifact}"; then
        error "Failed to download $url"
    fi

    # Extract
    cd "$temp_dir"
    if [[ "$artifact" == *.tar.gz ]]; then
        tar -xzf "$artifact"
    else
        unzip -q "$artifact"
    fi

    # Verify binary exists
    if [ ! -f "$binary_name" ]; then
        error "Binary not found in archive"
    fi

    # Install binary
    info "Installing to $INSTALL_DIR..."
    if [ -w "$INSTALL_DIR" ]; then
        mv "$binary_name" "$INSTALL_DIR/"
    else
        info "Requesting sudo permission to install to $INSTALL_DIR"
        sudo mv "$binary_name" "$INSTALL_DIR/"
    fi

    chmod +x "${INSTALL_DIR}/${binary_name}"

    echo ""
    info "brat v${version} installed successfully!"
    echo ""

    # Verify installation
    if command -v brat &> /dev/null; then
        info "Verification: $(brat --version 2>/dev/null || echo 'brat installed')"
    else
        warn "brat is installed but not in PATH."
        echo ""
        echo "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo ""
        echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
        echo ""
    fi
}

# Print banner
print_banner() {
    echo ""
    echo "  ____            _   "
    echo " | __ ) _ __ __ _| |_ "
    echo " |  _ \\| '__/ _\` | __|"
    echo " | |_) | | | (_| | |_ "
    echo " |____/|_|  \\__,_|\\__|"
    echo ""
    echo " Multi-agent Coding Orchestrator"
    echo ""
}

# Print usage
usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  -v, --version VERSION   Install specific version (default: latest)"
    echo "  -d, --dir DIRECTORY     Install to directory (default: /usr/local/bin)"
    echo "  -h, --help              Show this help message"
    echo ""
    echo "Environment variables:"
    echo "  BRAT_VERSION            Version to install"
    echo "  BRAT_INSTALL_DIR        Installation directory"
    echo "  BRAT_REPO               GitHub repository (default: YOUR_ORG/brat)"
    echo "  BRAT_DEBUG              Enable debug output (set to 1)"
    echo ""
    echo "Examples:"
    echo "  # Install latest version"
    echo "  curl -fsSL https://raw.githubusercontent.com/YOUR_ORG/brat/main/install.sh | bash"
    echo ""
    echo "  # Install specific version"
    echo "  curl -fsSL https://raw.githubusercontent.com/YOUR_ORG/brat/main/install.sh | BRAT_VERSION=0.1.0 bash"
    echo ""
    echo "  # Install to custom directory"
    echo "  curl -fsSL https://raw.githubusercontent.com/YOUR_ORG/brat/main/install.sh | BRAT_INSTALL_DIR=~/.local/bin bash"
    echo ""
}

# Parse arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -v|--version)
                VERSION="$2"
                shift 2
                ;;
            -d|--dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                ;;
        esac
    done
}

# Main
main() {
    parse_args "$@"
    print_banner
    check_deps
    install
}

main "$@"
