#!/bin/bash

# Image Viewer Development Script
# Usage: ./dev.sh [command]

set -e

case "$1" in
    "check")
        echo "🔍 Checking code..."
        cargo check --workspace --all-targets
        ;;
    "test")
        echo "🧪 Running tests..."
        cargo test --workspace
        ;;
    "run")
        echo "🚀 Running image viewer..."
        cargo run --release
        ;;
    "build")
        echo "🔨 Building release..."
        cargo build --release
        ;;
    "format")
        echo "🎨 Formatting code..."
        cargo fmt --all
        ;;
    "lint")
        echo "📏 Linting code..."
        cargo clippy --workspace --all-targets -- -D warnings
        ;;
    "clean")
        echo "🧹 Cleaning build artifacts..."
        cargo clean
        ;;
    "setup")
        echo "⚙️ Setting up development environment..."
        echo "Installing required tools..."
        rustup component add rustfmt clippy
        echo "✅ Development environment ready!"
        ;;
    "demo")
        echo "🖼️ Running with test images..."
        if [ -d "$HOME/Pictures" ]; then
            cargo run --release -- --test-dir "$HOME/Pictures"
        else
            cargo run --release
        fi
        ;;
    *)
        echo "🔧 Image Viewer Development Script"
        echo ""
        echo "Usage: $0 [command]"
        echo ""
        echo "Commands:"
        echo "  check   - Check code compilation"
        echo "  test    - Run tests"
        echo "  run     - Run the application"
        echo "  build   - Build release version"
        echo "  format  - Format code"
        echo "  lint    - Lint code"
        echo "  clean   - Clean build artifacts"
        echo "  setup   - Setup development environment"
        echo "  demo    - Run with test images directory"
        echo ""
        ;;
esac