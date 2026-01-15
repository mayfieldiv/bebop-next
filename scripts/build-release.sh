#!/usr/bin/env bash
#
# Build a release package for bebop.
#
# Usage:
#   ./scripts/build-release.sh [options]
#
# Options:
#   -v, --version VERSION   Override version (e.g., 1.2.3 or 1.2.3-rc1)
#   -o, --output DIR        Output directory (default: dist)
#   -j, --jobs N            Parallel build jobs (default: auto)
#   -c, --clean             Clean build directory before building
#   -s, --skip-vscode       Skip VSCode extension
#   -t, --skip-tests        Skip running tests
#   -h, --help              Show this help
#
# The version is determined in this order:
#   1. --version flag (highest priority)
#   2. Git tag on current commit (vX.Y.Z or vX.Y.Z-suffix)
#   3. VERSION file + git short hash (e.g., 0.1.0-dev+abc1234)
#
# Output structure:
#   dist/
#   ├── bin/
#   │   ├── bebopc
#   │   └── bebopc-gen-c
#   ├── lib/
#   │   └── libbebop.a
#   ├── include/
#   │   ├── bebop.h
#   │   └── bebop_wire.h
#   ├── share/
#   │   └── bebop/
#   │       └── *.bop (standard library schemas)
#   └── ide/
#       └── vscode/
#           └── bebop-X.Y.Z.vsix

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Defaults
OUTPUT_DIR="$PROJECT_ROOT/dist"
CLEAN_BUILD=false
SKIP_VSCODE=false
SKIP_TESTS=false
JOBS=""
VERSION_OVERRIDE=""

# Colors (disabled if not a terminal)
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    NC='\033[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' BOLD='' NC=''
fi

log_info()  { echo -e "${BLUE}==>${NC} $*"; }
log_ok()    { echo -e "${GREEN}==>${NC} $*"; }
log_warn()  { echo -e "${YELLOW}warning:${NC} $*"; }
log_error() { echo -e "${RED}error:${NC} $*" >&2; }
die()       { log_error "$@"; exit 1; }

usage() {
    sed -n '3,/^$/p' "$0" | sed 's/^# \?//'
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        -v|--version)
            VERSION_OVERRIDE="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -j|--jobs)
            JOBS="$2"
            shift 2
            ;;
        -c|--clean)
            CLEAN_BUILD=true
            shift
            ;;
        -s|--skip-vscode)
            SKIP_VSCODE=true
            shift
            ;;
        -t|--skip-tests)
            SKIP_TESTS=true
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            die "Unknown option: $1"
            ;;
    esac
done

# Determine version (returns version string, logs to stderr)
get_version() {
    local version=""
    local source=""

    # Priority 1: Command-line override
    if [[ -n "$VERSION_OVERRIDE" ]]; then
        version="$VERSION_OVERRIDE"
        source="override"
    # Priority 2: Git tag on current commit
    elif git -C "$PROJECT_ROOT" describe --tags --exact-match HEAD 2>/dev/null | grep -qE '^v[0-9]+\.[0-9]+\.[0-9]+'; then
        local tag
        tag=$(git -C "$PROJECT_ROOT" describe --tags --exact-match HEAD 2>/dev/null)
        version="${tag#v}"
        source="git tag"
    # Priority 3: VERSION file + dev suffix
    elif [[ -f "$PROJECT_ROOT/VERSION" ]]; then
        local base_version
        base_version=$(tr -d '[:space:]' < "$PROJECT_ROOT/VERSION")
        local short_hash
        short_hash=$(git -C "$PROJECT_ROOT" rev-parse --short HEAD 2>/dev/null || echo "unknown")
        version="${base_version}-dev+${short_hash}"
        source="VERSION file"
    else
        die "Cannot determine version: no --version, no git tag, no VERSION file"
    fi

    log_info "Version: $version ($source)" >&2
    echo "$version"
}

# Parse version string into components (sets global vars)
parse_version() {
    local ver="$1"
    # Match X.Y.Z or X.Y.Z-suffix
    if [[ "$ver" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-(.+))?$ ]]; then
        VERSION_MAJOR="${BASH_REMATCH[1]}"
        VERSION_MINOR="${BASH_REMATCH[2]}"
        VERSION_PATCH="${BASH_REMATCH[3]}"
        VERSION_SUFFIX="${BASH_REMATCH[5]:-}"
    else
        die "Invalid version format: $ver (expected X.Y.Z or X.Y.Z-suffix)"
    fi
}

# Detect number of CPUs
get_jobs() {
    if [[ -n "$JOBS" ]]; then
        echo "$JOBS"
    elif command -v nproc &>/dev/null; then
        nproc
    elif command -v sysctl &>/dev/null; then
        sysctl -n hw.ncpu
    else
        echo 4
    fi
}

# Build native components with CMake
build_native() {
    local build_dir="$PROJECT_ROOT/build-release"
    local jobs
    jobs=$(get_jobs)

    log_info "Building native components (jobs=$jobs)"

    if [[ "$CLEAN_BUILD" == true ]] && [[ -d "$build_dir" ]]; then
        log_info "Cleaning build directory"
        rm -rf "$build_dir"
    fi

    mkdir -p "$build_dir"

    cmake -B "$build_dir" -S "$PROJECT_ROOT" \
        -DCMAKE_BUILD_TYPE=Release \
        -DBEBOP_BUILD_TESTS=ON \
        -DCMAKE_INSTALL_PREFIX="$OUTPUT_DIR" \
        -DBEBOP_VERSION_MAJOR="$VERSION_MAJOR" \
        -DBEBOP_VERSION_MINOR="$VERSION_MINOR" \
        -DBEBOP_VERSION_PATCH="$VERSION_PATCH" \
        -DBEBOP_VERSION_SUFFIX="$VERSION_SUFFIX"

    cmake --build "$build_dir" --parallel "$jobs"

    if [[ "$SKIP_TESTS" != true ]]; then
        log_info "Running tests"
        ctest --test-dir "$build_dir" --output-on-failure
    fi

    log_ok "Native build complete"
}

# Build VSCode extension
build_vscode() {
    if [[ "$SKIP_VSCODE" == true ]]; then
        log_info "Skipping VSCode extension"
        return
    fi

    local vscode_dir="$PROJECT_ROOT/plugins/vscode"

    if [[ ! -d "$vscode_dir" ]]; then
        log_warn "VSCode extension directory not found, skipping"
        return
    fi

    log_info "Building VSCode extension"

    cd "$vscode_dir"

    # Install dependencies
    if [[ ! -d "node_modules" ]] || [[ ! -d "client/node_modules" ]]; then
        log_info "Installing npm dependencies"
        npm install
    fi

    # Update version in package.json if needed
    local pkg_version
    pkg_version=$(jq -r .version package.json)
    if [[ "$pkg_version" != "$VERSION" ]]; then
        log_info "Updating package.json version to $VERSION"
        local tmp
        tmp=$(mktemp)
        jq --arg v "$VERSION" '.version = $v' package.json > "$tmp" && mv "$tmp" package.json
    fi

    # Build
    npm run compile

    # Package
    npm run package

    cd "$PROJECT_ROOT"
    log_ok "VSCode extension built"
}

# Stage release artifacts
stage_release() {
    local build_dir="$PROJECT_ROOT/build-release"

    log_info "Staging release to $OUTPUT_DIR"

    rm -rf "$OUTPUT_DIR"
    mkdir -p "$OUTPUT_DIR"/{bin,lib,include,share/bebop,ide/vscode}

    # Binaries
    cp "$build_dir/bebopc/bebopc" "$OUTPUT_DIR/bin/"

    # Plugins - collect all bebopc-gen-* executables
    for plugin in "$build_dir/bebopc"/bebopc-gen-*; do
        if [[ -f "$plugin" ]] && [[ -x "$plugin" ]]; then
            local name
            name=$(basename "$plugin")
            cp "$plugin" "$OUTPUT_DIR/bin/"
            log_info "  Staged plugin: $name"
        fi
    done

    # Library
    cp "$build_dir/bebop/libbebop.a" "$OUTPUT_DIR/lib/"

    # Headers
    cp "$PROJECT_ROOT/bebop/include/bebop.h" "$OUTPUT_DIR/include/"
    cp "$PROJECT_ROOT/bebop/include/bebop_wire.h" "$OUTPUT_DIR/include/"

    # Standard library schemas
    local schema_src=""
    if [[ -d "$build_dir/schemas/bebop" ]]; then
        schema_src="$build_dir/schemas/bebop"
    elif [[ -d "$PROJECT_ROOT/bebop/schemas/bebop" ]]; then
        schema_src="$PROJECT_ROOT/bebop/schemas/bebop"
    fi

    if [[ -n "$schema_src" ]] && [[ -n "$(ls -A "$schema_src"/*.bop 2>/dev/null)" ]]; then
        cp "$schema_src"/*.bop "$OUTPUT_DIR/share/bebop/"
        log_info "  Staged $(ls -1 "$schema_src"/*.bop | wc -l | tr -d ' ') schema files"
    else
        log_warn "No standard library schemas found"
    fi

    # VSCode extension
    if [[ "$SKIP_VSCODE" != true ]]; then
        local vsix
        vsix=$(find "$PROJECT_ROOT/plugins/vscode" -maxdepth 1 -name "*.vsix" -type f | head -1)
        if [[ -n "$vsix" ]]; then
            cp "$vsix" "$OUTPUT_DIR/ide/vscode/"
            log_info "  Staged VSCode extension: $(basename "$vsix")"
        fi
    fi

    log_ok "Release staged"
}

# Create archive
create_archive() {
    local archive_name="bebop-${VERSION}-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)"
    local archive_path="$PROJECT_ROOT/${archive_name}.tar.gz"

    log_info "Creating archive: $archive_name.tar.gz"

    tar -czf "$archive_path" -C "$OUTPUT_DIR" .

    log_ok "Archive created: $archive_path"
    echo ""
    echo "Release contents:"
    tar -tzf "$archive_path" | head -30
    if [[ $(tar -tzf "$archive_path" | wc -l) -gt 30 ]]; then
        echo "  ... (truncated)"
    fi
}

# Summary
print_summary() {
    echo ""
    echo -e "${BOLD}Release Summary${NC}"
    echo "  Version:  $VERSION"
    echo "  Output:   $OUTPUT_DIR"
    echo ""
    echo "Contents:"
    find "$OUTPUT_DIR" -type f | sed "s|$OUTPUT_DIR/|  |" | sort
    echo ""
    log_ok "Build complete"
}

# Main
main() {
    cd "$PROJECT_ROOT"

    echo ""
    echo -e "${BOLD}Bebop Release Build${NC}"
    echo ""

    # Determine version early so we can use it everywhere
    VERSION=$(get_version)
    parse_version "$VERSION"
    export VERSION VERSION_MAJOR VERSION_MINOR VERSION_PATCH VERSION_SUFFIX

    # Validate we're in a clean state for tagged releases
    if [[ -z "$VERSION_OVERRIDE" ]] && git -C "$PROJECT_ROOT" describe --tags --exact-match HEAD &>/dev/null; then
        if [[ -n "$(git -C "$PROJECT_ROOT" status --porcelain)" ]]; then
            log_warn "Working directory has uncommitted changes"
        fi
    fi

    build_native
    build_vscode
    stage_release
    create_archive
    print_summary
}

main "$@"
