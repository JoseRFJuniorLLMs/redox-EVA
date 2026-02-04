#!/bin/bash
# Setup script for EVA Daemon development

echo "🧠 EVA Daemon Setup Script"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Run this script from the eva-daemon directory"
    exit 1
fi

# Function to setup Phase 1
setup_phase1() {
    echo ""
    echo "📦 Setting up Phase 1 (Network Testing)..."
    cp Cargo_phase1.toml Cargo.toml
    cp src/main_phase1.rs src/main.rs
    echo "✅ Phase 1 configured"
    echo ""
    echo "To test Phase 1:"
    echo "  cargo build --release"
    echo "  cargo run"
}

# Function to setup Phase 2
setup_phase2() {
    echo ""
    echo "🔐 Setting up Phase 2 (TLS/SSL)..."
    # Cargo.toml is already Phase 2 by default
    # main.rs is already Phase 2 by default
    echo "✅ Phase 2 configured (default)"
    echo ""
    echo "To test Phase 2:"
    echo "  cargo build --release"
    echo "  cargo test"
    echo "  cargo run"
}

# Main menu
echo ""
echo "Select phase to setup:"
echo "  1) Phase 1 - Network Testing"
echo "  2) Phase 2 - TLS/SSL (default)"
echo ""
read -p "Enter choice [1-2]: " choice

case $choice in
    1)
        setup_phase1
        ;;
    2)
        setup_phase2
        ;;
    *)
        echo "Invalid choice, setting up Phase 2 (default)"
        setup_phase2
        ;;
esac

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Setup complete!"
