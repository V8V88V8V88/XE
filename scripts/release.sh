#!/usr/bin/env bash
set -e

# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.1.5

if [ -z "$1" ]; then
    echo "Error: No version specified."
    echo "Usage: ./scripts/release.sh <version> (e.g., 0.1.5)"
    exit 1
fi

VERSION="${1#v}"
TAG="v$VERSION"

echo "==> Preparing release $TAG..."

# 1. Check if git working directory is clean
if [ -n "$(git status --porcelain)" ]; then
    echo "==> Staging existing changes..."
fi

# 2. Run full test suite
echo "==> Running test suite..."
cargo test --quiet

# 3. Update Cargo.toml
echo "==> Updating version in Cargo.toml to $VERSION..."
sed -i -E "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml

# 4. Sync Cargo.lock
echo "==> Updating Cargo.lock..."
cargo check --quiet

# 5. Update Documentation references
echo "==> Updating documentation references..."
sed -i -E "s/\*\*Current Version:\*\* .* pre-alpha/**Current Version:** $VERSION pre-alpha/" README.md 2>/dev/null || true
sed -i -E "s/XE_VERSION=\"v[^\"]*\"/XE_VERSION=\"$TAG\"/" docs/guide/getting-started.md 2>/dev/null || true
sed -i -E "s/XE is in \*\*pre-alpha\*\* \([^)]*\)/XE is in **pre-alpha** ($TAG)/" docs/reference/status.md 2>/dev/null || true

# 6. Commit changes
echo "==> Creating commit and tag..."
git add Cargo.toml Cargo.lock README.md docs/ .github/
git commit -m "chore: release $TAG" || true

# 7. Tag the release
git tag -a "$TAG" -m "Release $TAG"

echo ""
echo "🎉 Release $TAG is ready locally!"
echo "To publish to GitHub, run:"
echo "    git push origin main && git push origin $TAG"
