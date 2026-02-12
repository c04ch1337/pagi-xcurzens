#!/usr/bin/env bash
# Phoenix Release Trigger Script (Bash)
# This script creates and pushes a git tag to trigger the GitHub Actions release workflow

set -e

VERSION="${1:-}"

# Read version from VERSION file if not provided
if [ -z "$VERSION" ]; then
    if [ -f "VERSION" ]; then
        VERSION=$(cat VERSION | tr -d '[:space:]')
        echo -e "\033[36m📦 Using version from VERSION file: $VERSION\033[0m"
    else
        echo -e "\033[31m❌ VERSION file not found and no version specified\033[0m"
        echo -e "\033[33mUsage: ./trigger-release.sh [version]\033[0m"
        exit 1
    fi
fi

# Validate version format
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    echo -e "\033[31m❌ Invalid version format: $VERSION\033[0m"
    echo -e "\033[33mExpected format: X.Y.Z or X.Y.Z-beta.N\033[0m"
    exit 1
fi

TAG_NAME="v$VERSION"

echo ""
echo -e "\033[35m🚀 Phoenix Release Trigger\033[0m"
echo -e "\033[90m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"
echo ""
echo -e "  Version: \033[37m$VERSION\033[0m"
echo -e "  Tag:     \033[37m$TAG_NAME\033[0m"
echo ""

# Check if tag already exists
if git rev-parse "$TAG_NAME" >/dev/null 2>&1; then
    echo -e "\033[33m⚠️  Tag $TAG_NAME already exists locally\033[0m"
    read -p "Delete and recreate? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "\033[31m❌ Aborted\033[0m"
        exit 1
    fi
    git tag -d "$TAG_NAME"
    echo -e "\033[32m✓ Deleted local tag\033[0m"
fi

# Check git status
if [ -n "$(git status --porcelain)" ]; then
    echo -e "\033[33m⚠️  You have uncommitted changes:\033[0m"
    git status --porcelain | sed 's/^/  /'
    echo ""
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "\033[31m❌ Aborted\033[0m"
        exit 1
    fi
fi

echo ""
echo -e "\033[90m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"
echo -e "\033[36m⚡ This will:\033[0m"
echo -e "   1. Create git tag: \033[37m$TAG_NAME\033[0m"
echo -e "   2. Push tag to origin"
echo -e "   3. Trigger GitHub Actions release workflow"
echo -e "   4. Build binaries for 4 platforms (Windows, Linux, macOS x2)"
echo -e "   5. Create GitHub Release with artifacts"
echo -e "\033[90m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"
echo ""

read -p "Proceed with release? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "\033[31m❌ Aborted\033[0m"
    exit 1
fi

echo ""
echo -e "\033[36m🏷️  Creating tag...\033[0m"
git tag -a "$TAG_NAME" -m "Phoenix Release v$VERSION"
echo -e "\033[32m✓ Tag created\033[0m"

echo ""
echo -e "\033[36m📤 Pushing tag to origin...\033[0m"
if ! git push origin "$TAG_NAME"; then
    echo -e "\033[31m❌ Failed to push tag\033[0m"
    echo -e "\033[33m   You may need to delete the remote tag first:\033[0m"
    echo -e "\033[90m   git push origin :refs/tags/$TAG_NAME\033[0m"
    exit 1
fi

echo -e "\033[32m✓ Tag pushed\033[0m"
echo ""
echo -e "\033[90m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"
echo -e "\033[35m🔥 Phoenix Release Triggered!\033[0m"
echo ""
echo -e "\033[36mMonitor the build at:\033[0m"
echo -e "\033[34mhttps://github.com/YOUR_USERNAME/YOUR_REPO/actions\033[0m"
echo ""
echo -e "\033[37mThe workflow will:\033[0m"
echo -e "  \033[90m• Build for 4 platforms (~10-15 minutes)\033[0m"
echo -e "  \033[90m• Generate SHA256 checksums\033[0m"
echo -e "  \033[90m• Create GitHub Release with artifacts\033[0m"
echo -e "  \033[90m• Bundle QUICKSTART.md and ONBOARDING_GUIDE.md\033[0m"
echo ""
echo -e "\033[90m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"
