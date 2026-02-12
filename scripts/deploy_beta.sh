#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════════════════════
# Phoenix Beta Deployment Script
# ══════════════════════════════════════════════════════════════════════════════
# Purpose: Build release binaries and prepare for beta distribution
# Usage: ./scripts/deploy_beta.sh
# ══════════════════════════════════════════════════════════════════════════════

set -e

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}🚀 Phoenix Beta Deployment Pipeline${NC}"
echo "════════════════════════════════════════════════════════"

# Read version from VERSION file
if [ ! -f "VERSION" ]; then
    echo -e "${RED}❌ VERSION file not found!${NC}"
    exit 1
fi

VERSION=$(cat VERSION | tr -d '[:space:]')
echo -e "${BLUE}Version:${NC} $VERSION"
echo ""

# Step 1: Sanitize
echo -e "${YELLOW}Step 1: Sanitizing repository...${NC}"
if [ -f "scripts/sanitize_for_release.sh" ]; then
    bash scripts/sanitize_for_release.sh
else
    echo -e "${RED}❌ Sanitization script not found!${NC}"
    exit 1
fi
echo ""

# Step 2: Run tests
echo -e "${YELLOW}Step 2: Running tests...${NC}"
cargo test --workspace --release || {
    echo -e "${RED}❌ Tests failed! Fix issues before deploying.${NC}"
    exit 1
}
echo -e "${GREEN}✓ All tests passed${NC}"
echo ""

# Step 3: Build release binaries
echo -e "${YELLOW}Step 3: Building release binaries...${NC}"
echo "This may take several minutes..."

# Build the main gateway
cargo build --release -p pagi-gateway || {
    echo -e "${RED}❌ Build failed!${NC}"
    exit 1
}

# Build additional components
cargo build --release -p pagi-daemon || true
cargo build --release -p pagi-studio-ui || true

echo -e "${GREEN}✓ Build complete${NC}"
echo ""

# Step 4: Create release directory
echo -e "${YELLOW}Step 4: Preparing release package...${NC}"
RELEASE_DIR="releases/phoenix-${VERSION}"
mkdir -p "$RELEASE_DIR"

# Copy binaries
echo "Copying binaries..."
cp target/release/pagi-gateway "$RELEASE_DIR/" || cp target/release/pagi-gateway.exe "$RELEASE_DIR/" 2>/dev/null || true
cp target/release/pagi-daemon "$RELEASE_DIR/" 2>/dev/null || true
cp target/release/pagi-daemon.exe "$RELEASE_DIR/" 2>/dev/null || true

# Copy essential files
echo "Copying documentation and configuration..."
cp VERSION "$RELEASE_DIR/"
cp README.md "$RELEASE_DIR/"
cp .env.example "$RELEASE_DIR/"
cp -r scripts "$RELEASE_DIR/" 2>/dev/null || true

# Copy startup scripts
cp phoenix-rise.sh "$RELEASE_DIR/" 2>/dev/null || true
cp phoenix-rise.ps1 "$RELEASE_DIR/" 2>/dev/null || true
cp pagi-up.sh "$RELEASE_DIR/" 2>/dev/null || true
cp pagi-up.ps1 "$RELEASE_DIR/" 2>/dev/null || true

# Create README for beta users
cat > "$RELEASE_DIR/BETA_README.md" << 'EOF'
# Phoenix Beta Release

Welcome to the Phoenix Beta! This is your personal AI companion with sovereign intelligence.

## 🚀 Quick Start

### First Time Setup

1. **Configure your API key:**
   ```bash
   # Copy the example environment file
   cp .env.example .env
   
   # Edit .env and add your OpenRouter API key
   # Get one at: https://openrouter.ai/keys
   ```

2. **Start Phoenix:**
   ```bash
   # On Linux/Mac:
   ./phoenix-rise.sh
   
   # On Windows:
   .\phoenix-rise.ps1
   ```

3. **Access the UI:**
   Open your browser to `http://localhost:3001`

### What Happens on First Run?

- Phoenix will initialize empty knowledge bases (KB-01 through KB-08)
- You'll be prompted to provide your OpenRouter API key via the UI
- Your personal data stays on YOUR machine - nothing is sent to external servers
- As you interact, Phoenix learns about YOU and builds your personal knowledge graph

## 📚 Documentation

- `README.md` - Full project documentation
- `.env.example` - Configuration options
- `VERSION` - Current release version

## 🔐 Privacy & Security

- All your data is stored locally in the `storage/` and `vector_db/` directories
- Your API keys are stored in `user_config.toml` (never committed to git)
- Phoenix never sends your personal data to external services
- Only LLM API calls go through OpenRouter (using YOUR key)

## 🆘 Support

For issues, questions, or feedback, please open an issue on GitHub.

## 📝 License

See LICENSE file in the repository.
EOF

echo -e "${GREEN}✓ Release package created: $RELEASE_DIR${NC}"
echo ""

# Step 5: Create archive
echo -e "${YELLOW}Step 5: Creating release archive...${NC}"
cd releases
tar -czf "phoenix-${VERSION}.tar.gz" "phoenix-${VERSION}/" || {
    echo -e "${YELLOW}⚠ tar failed, trying zip...${NC}"
    zip -r "phoenix-${VERSION}.zip" "phoenix-${VERSION}/" || {
        echo -e "${RED}❌ Failed to create archive${NC}"
        exit 1
    }
}
cd ..

echo -e "${GREEN}✓ Archive created${NC}"
echo ""

# Step 6: Generate checksums
echo -e "${YELLOW}Step 6: Generating checksums...${NC}"
cd releases
if command -v sha256sum &> /dev/null; then
    sha256sum phoenix-${VERSION}.tar.gz > phoenix-${VERSION}.tar.gz.sha256 2>/dev/null || true
    sha256sum phoenix-${VERSION}.zip > phoenix-${VERSION}.zip.sha256 2>/dev/null || true
elif command -v shasum &> /dev/null; then
    shasum -a 256 phoenix-${VERSION}.tar.gz > phoenix-${VERSION}.tar.gz.sha256 2>/dev/null || true
    shasum -a 256 phoenix-${VERSION}.zip > phoenix-${VERSION}.zip.sha256 2>/dev/null || true
fi
cd ..

echo -e "${GREEN}✓ Checksums generated${NC}"
echo ""

# Summary
echo "════════════════════════════════════════════════════════"
echo -e "${GREEN}✨ Beta Deployment Complete!${NC}"
echo ""
echo "Release artifacts:"
echo "  📦 releases/phoenix-${VERSION}/"
echo "  📦 releases/phoenix-${VERSION}.tar.gz"
echo "  📦 releases/phoenix-${VERSION}.zip"
echo ""
echo "Next steps:"
echo "  1. Test the release package locally"
echo "  2. Create a GitHub Release with tag v${VERSION}"
echo "  3. Upload the archives to the release"
echo "  4. Share with beta testers!"
echo ""
echo "GitHub Release Command:"
echo -e "${CYAN}  gh release create v${VERSION} releases/phoenix-${VERSION}.tar.gz releases/phoenix-${VERSION}.zip --title \"Phoenix v${VERSION}\" --notes \"Beta release\"${NC}"
echo "════════════════════════════════════════════════════════"
