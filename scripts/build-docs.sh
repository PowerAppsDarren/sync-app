#!/bin/bash
# build-docs.sh - Build and test documentation with mdBook

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BOOK_DIR="."
DOCS_DIR="docs"
BUILD_DIR="docs-site"
SERVE_PORT="3001"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if mdBook is installed
check_mdbook() {
    if ! command -v mdbook &> /dev/null; then
        print_error "mdBook is not installed. Please install it first:"
        echo "  cargo install mdbook"
        echo "  # Or download from: https://github.com/rust-lang/mdBook/releases"
        exit 1
    fi
    
    print_success "mdBook found: $(mdbook --version)"
}

# Install useful mdBook plugins
install_plugins() {
    print_status "Installing mdBook plugins..."
    
    # Core plugins
    if ! command -v mdbook-linkcheck &> /dev/null; then
        print_status "Installing mdbook-linkcheck..."
        cargo install mdbook-linkcheck
    fi
    
    if ! command -v mdbook-toc &> /dev/null; then
        print_status "Installing mdbook-toc..."
        cargo install mdbook-toc
    fi
    
    if ! command -v mdbook-mermaid &> /dev/null; then
        print_status "Installing mdbook-mermaid..."
        cargo install mdbook-mermaid
    fi
    
    print_success "Plugins installed"
}

# Validate documentation structure
validate_structure() {
    print_status "Validating documentation structure..."
    
    # Check required files
    required_files=(
        "book.toml"
        "docs/SUMMARY.md"
        "docs/README.md"
        "docs/quick-start.md"
        "docs/configuration.md"
        "docs/deployment.md"
        "docs/troubleshooting.md"
    )
    
    for file in "${required_files[@]}"; do
        if [ ! -f "$file" ]; then
            print_error "Required file missing: $file"
            exit 1
        fi
    done
    
    # Check example files
    example_files=(
        "docs/examples/one-way-mirror.md"
        "docs/examples/bidirectional-cron.md"
        "docs/examples/watcher-demo.md"
    )
    
    for file in "${example_files[@]}"; do
        if [ ! -f "$file" ]; then
            print_warning "Example file missing: $file"
        fi
    done
    
    print_success "Documentation structure valid"
}

# Build documentation
build_docs() {
    print_status "Building documentation..."
    
    # Clean previous build
    if [ -d "$BUILD_DIR" ]; then
        rm -rf "$BUILD_DIR"
    fi
    
    # Build with mdBook
    mdbook build
    
    if [ $? -eq 0 ]; then
        print_success "Documentation built successfully"
        print_status "Output directory: $BUILD_DIR"
        print_status "Size: $(du -sh $BUILD_DIR | cut -f1)"
    else
        print_error "Documentation build failed"
        exit 1
    fi
}

# Test documentation
test_docs() {
    print_status "Testing documentation..."
    
    # Test with mdBook
    mdbook test
    
    if [ $? -eq 0 ]; then
        print_success "Documentation tests passed"
    else
        print_error "Documentation tests failed"
        exit 1
    fi
}

# Check links
check_links() {
    if command -v mdbook-linkcheck &> /dev/null; then
        print_status "Checking links..."
        
        # Run link checker
        mdbook-linkcheck
        
        if [ $? -eq 0 ]; then
            print_success "All links are valid"
        else
            print_warning "Some links may be broken (check output above)"
        fi
    else
        print_warning "mdbook-linkcheck not available, skipping link validation"
    fi
}

# Serve documentation locally
serve_docs() {
    print_status "Starting local server on port $SERVE_PORT..."
    print_status "Open http://localhost:$SERVE_PORT in your browser"
    print_status "Press Ctrl+C to stop"
    
    mdbook serve --port "$SERVE_PORT" --open
}

# Generate documentation statistics
generate_stats() {
    print_status "Generating documentation statistics..."
    
    if [ ! -d "$DOCS_DIR" ]; then
        print_error "Documentation directory not found: $DOCS_DIR"
        return 1
    fi
    
    # Count files and content
    total_files=$(find "$DOCS_DIR" -name "*.md" | wc -l)
    total_lines=$(find "$DOCS_DIR" -name "*.md" -exec wc -l {} + | tail -1 | awk '{print $1}')
    total_words=$(find "$DOCS_DIR" -name "*.md" -exec wc -w {} + | tail -1 | awk '{print $1}')
    
    echo ""
    echo "📊 Documentation Statistics:"
    echo "  Total Markdown files: $total_files"
    echo "  Total lines: $total_lines"
    echo "  Total words: $total_words"
    echo "  Estimated reading time: $((total_words / 200)) minutes"
    
    # List largest files
    echo ""
    echo "📄 Largest documentation files:"
    find "$DOCS_DIR" -name "*.md" -exec wc -l {} + | sort -nr | head -5 | while read lines file; do
        echo "  $lines lines - $(basename "$file")"
    done
    
    # Check for missing sections in SUMMARY.md
    echo ""
    echo "🔍 Coverage Analysis:"
    missing_files=()
    while IFS= read -r file; do
        relative_path=$(realpath --relative-to="$DOCS_DIR" "$file")
        if ! grep -q "$relative_path" "$DOCS_DIR/SUMMARY.md"; then
            missing_files+=("$relative_path")
        fi
    done < <(find "$DOCS_DIR" -name "*.md" ! -name "SUMMARY.md")
    
    if [ ${#missing_files[@]} -eq 0 ]; then
        print_success "All markdown files are referenced in SUMMARY.md"
    else
        print_warning "Files not referenced in SUMMARY.md:"
        for file in "${missing_files[@]}"; do
            echo "  - $file"
        done
    fi
}

# Spell check (if available)
spell_check() {
    if command -v aspell &> /dev/null; then
        print_status "Running spell check..."
        
        # Create temporary file for spell check results
        spell_errors=$(mktemp)
        
        find "$DOCS_DIR" -name "*.md" -exec aspell list -d en_US < {} \; | sort | uniq > "$spell_errors"
        
        if [ -s "$spell_errors" ]; then
            print_warning "Possible spelling errors found:"
            cat "$spell_errors" | head -20
            if [ $(wc -l < "$spell_errors") -gt 20 ]; then
                echo "  ... and $(( $(wc -l < "$spell_errors") - 20 )) more"
            fi
        else
            print_success "No spelling errors found"
        fi
        
        rm -f "$spell_errors"
    else
        print_status "aspell not available, skipping spell check"
        print_status "Install with: sudo apt install aspell aspell-en"
    fi
}

# Main function
main() {
    case "${1:-build}" in
        "install")
            check_mdbook
            install_plugins
            ;;
        "validate")
            validate_structure
            ;;
        "build")
            check_mdbook
            validate_structure
            build_docs
            ;;
        "test")
            check_mdbook
            test_docs
            check_links
            ;;
        "serve")
            check_mdbook
            build_docs
            serve_docs
            ;;
        "stats")
            generate_stats
            ;;
        "spell")
            spell_check
            ;;
        "all")
            check_mdbook
            install_plugins
            validate_structure
            build_docs
            test_docs
            check_links
            generate_stats
            print_success "All documentation tasks completed!"
            ;;
        "clean")
            print_status "Cleaning build directory..."
            rm -rf "$BUILD_DIR"
            print_success "Build directory cleaned"
            ;;
        "help"|*)
            echo "Usage: $0 [command]"
            echo ""
            echo "Commands:"
            echo "  install   - Install mdBook and plugins"
            echo "  validate  - Validate documentation structure"
            echo "  build     - Build documentation"
            echo "  test      - Test documentation and links"
            echo "  serve     - Build and serve documentation locally"
            echo "  stats     - Generate documentation statistics"
            echo "  spell     - Run spell check (requires aspell)"
            echo "  all       - Run all tasks (install, build, test, stats)"
            echo "  clean     - Clean build directory"
            echo "  help      - Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0 build     # Build documentation"
            echo "  $0 serve     # Build and serve locally"
            echo "  $0 all       # Complete build and test cycle"
            ;;
    esac
}

# Run main function with all arguments
main "$@"
