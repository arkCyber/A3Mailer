#!/bin/bash
# A3Mailer Development Helper Script
# This script provides common development tasks

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Project root directory
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

# Logging function
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}"
    exit 1
}

# Check if required tools are installed
check_dependencies() {
    log "Checking dependencies..."
    
    local deps=("docker" "docker-compose" "cargo" "rustc")
    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null; then
            error "$dep is not installed or not in PATH"
        fi
    done
    
    log "All dependencies are available"
}

# Setup development environment
setup() {
    log "Setting up development environment..."
    
    # Create necessary directories
    mkdir -p logs data config
    
    # Install Rust tools if not present
    if ! cargo install --list | grep -q cargo-watch; then
        log "Installing cargo-watch..."
        cargo install cargo-watch
    fi
    
    if ! cargo install --list | grep -q cargo-audit; then
        log "Installing cargo-audit..."
        cargo install cargo-audit
    fi
    
    # Copy example configuration
    if [ ! -f "config/development.toml" ]; then
        log "Creating development configuration..."
        cp resources/config/spamfilter.toml config/development.toml
    fi
    
    log "Development environment setup complete"
}

# Build the project
build() {
    log "Building A3Mailer..."
    cargo build --workspace
    log "Build complete"
}

# Build release version
build_release() {
    log "Building A3Mailer (release)..."
    cargo build --release --workspace
    log "Release build complete"
}

# Run tests
test() {
    log "Running tests..."
    cargo test --workspace
    log "Tests complete"
}

# Run tests with coverage
test_coverage() {
    log "Running tests with coverage..."
    
    if ! cargo install --list | grep -q cargo-tarpaulin; then
        log "Installing cargo-tarpaulin..."
        cargo install cargo-tarpaulin
    fi
    
    cargo tarpaulin --workspace --out Html --output-dir coverage
    log "Coverage report generated in coverage/"
}

# Run linting
lint() {
    log "Running linting..."
    
    # Format check
    cargo fmt --all -- --check
    
    # Clippy
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    
    # Audit
    cargo audit
    
    log "Linting complete"
}

# Format code
format() {
    log "Formatting code..."
    cargo fmt --all
    log "Code formatted"
}

# Start development server
dev() {
    log "Starting development server..."
    cargo watch -x "run --bin a3mailer -- --config config/development.toml"
}

# Start with Docker Compose
docker_dev() {
    log "Starting development environment with Docker..."
    docker-compose -f docker-compose.yml -f docker-compose.dev.yml up --build
}

# Stop Docker Compose
docker_stop() {
    log "Stopping Docker development environment..."
    docker-compose -f docker-compose.yml -f docker-compose.dev.yml down
}

# Clean build artifacts
clean() {
    log "Cleaning build artifacts..."
    cargo clean
    docker system prune -f
    log "Clean complete"
}

# Show help
help() {
    cat << EOF
A3Mailer Development Helper Script

Usage: $0 <command>

Commands:
    setup           Setup development environment
    build           Build the project
    build-release   Build release version
    test            Run tests
    test-coverage   Run tests with coverage
    lint            Run linting (fmt, clippy, audit)
    format          Format code
    dev             Start development server
    docker-dev      Start with Docker Compose
    docker-stop     Stop Docker Compose
    clean           Clean build artifacts
    help            Show this help

Examples:
    $0 setup        # First time setup
    $0 dev          # Start development server
    $0 test         # Run all tests
    $0 lint         # Check code quality

EOF
}

# Main command dispatcher
main() {
    case "${1:-help}" in
        setup)
            check_dependencies
            setup
            ;;
        build)
            build
            ;;
        build-release)
            build_release
            ;;
        test)
            test
            ;;
        test-coverage)
            test_coverage
            ;;
        lint)
            lint
            ;;
        format)
            format
            ;;
        dev)
            dev
            ;;
        docker-dev)
            docker_dev
            ;;
        docker-stop)
            docker_stop
            ;;
        clean)
            clean
            ;;
        help|--help|-h)
            help
            ;;
        *)
            error "Unknown command: $1. Use '$0 help' for usage information."
            ;;
    esac
}

# Run main function with all arguments
main "$@"
