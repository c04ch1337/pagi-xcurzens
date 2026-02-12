#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════════════════════
# Phoenix Release Sanitization Script
# ══════════════════════════════════════════════════════════════════════════════
# Purpose: Remove all personal "biological" data before beta distribution
# Usage: ./scripts/sanitize_for_release.sh
# ══════════════════════════════════════════════════════════════════════════════

set -e

echo "🔥 Phoenix Release Sanitization - Removing Personal Data"
echo "════════════════════════════════════════════════════════"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to safely remove directory
safe_remove_dir() {
    local dir=$1
    if [ -d "$dir" ]; then
        echo -e "${YELLOW}Removing:${NC} $dir"
        rm -rf "$dir"
        echo -e "${GREEN}✓ Removed${NC}"
    else
        echo -e "${GREEN}✓ Already clean:${NC} $dir"
    fi
}

# Function to safely remove file
safe_remove_file() {
    local file=$1
    if [ -f "$file" ]; then
        echo -e "${YELLOW}Removing:${NC} $file"
        rm -f "$file"
        echo -e "${GREEN}✓ Removed${NC}"
    else
        echo -e "${GREEN}✓ Already clean:${NC} $file"
    fi
}

echo ""
echo "📦 Removing Vector Databases (KB-01 through KB-08)..."
safe_remove_dir "storage"
safe_remove_dir "vector_db"
find . -type d -name "*.sled" -exec rm -rf {} + 2>/dev/null || true

echo ""
echo "🗄️ Removing Gateway Runtime Data..."
safe_remove_dir "data"
safe_remove_dir "pagi-gateway/data"
safe_remove_dir "add-ons/pagi-gateway/data"

echo ""
echo "🔐 Removing Environment Files..."
safe_remove_file ".env"
safe_remove_file ".env.local"
find . -type f -name ".env.*.local" -exec rm -f {} + 2>/dev/null || true

echo ""
echo "⚙️ Removing User Configuration..."
safe_remove_file "user_config.toml"
safe_remove_file "config/user_config.toml"

echo ""
echo "🧹 Removing Build Artifacts..."
safe_remove_dir "target"

echo ""
echo "📝 Removing Logs..."
find . -type f -name "*.log" -exec rm -f {} + 2>/dev/null || true
safe_remove_dir "logs"

echo ""
echo "🗑️ Removing Qdrant Binary (users will download their own)..."
safe_remove_dir "qdrant"
safe_remove_file "qdrant.zip"

echo ""
echo "🧪 Removing Research Sandbox Content..."
if [ -d "research_sandbox" ]; then
    # Keep the directory structure but remove user content
    find research_sandbox -type f ! -name "README.md" ! -name ".gitkeep" -exec rm -f {} + 2>/dev/null || true
    echo -e "${GREEN}✓ Cleaned research_sandbox${NC}"
fi

echo ""
echo "════════════════════════════════════════════════════════"
echo -e "${GREEN}✨ Sanitization Complete!${NC}"
echo ""
echo "The repository is now ready for beta distribution."
echo "All personal data has been removed while preserving the genetic code."
echo ""
echo "Next steps:"
echo "  1. Review changes: git status"
echo "  2. Test clean build: cargo build --release"
echo "  3. Run deploy script: ./scripts/deploy_beta.sh"
echo "════════════════════════════════════════════════════════"
