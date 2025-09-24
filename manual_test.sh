#!/bin/bash

# Manual test script for iconmate add command
# This script tests the exact command you mentioned:
# iconmate add --folder ./src/assets/icons/ --icon heroicons:heart --name Heart

set -e  # Exit on any error

echo "🔧 Building iconmate binary..."
cargo build --release

echo "📁 Creating test directory..."
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"
echo "Test directory: $TEST_DIR"

echo "🚀 Running iconmate add command..."
"$OLDPWD/target/debug/iconmate" add --folder ./src/assets/icons/ --icon heroicons:heart --name Heart

echo "📋 Verifying results..."
echo "1. Checking if src/assets/icons/ directory exists..."
if [ -d "src/assets/icons/" ]; then
    echo "✅ Directory src/assets/icons/ exists"
else
    echo "❌ Directory src/assets/icons/ does not exist"
    exit 1
fi

echo "2. Checking if index.ts file exists..."
if [ -f "src/assets/icons/index.ts" ]; then
    echo "✅ File index.ts exists"
    echo "   Content:"
    cat src/assets/icons/index.ts
else
    echo "❌ File index.ts does not exist"
    exit 1
fi

echo "3. Checking if heroicons:heart.svg file exists..."
if [ -f "src/assets/icons/heroicons:heart.svg" ]; then
    echo "✅ File heroicons:heart.svg exists"
    echo "   SVG size: $(wc -c < src/assets/icons/heroicons:heart.svg) bytes"
    echo "   First line: $(head -n 1 src/assets/icons/heroicons:heart.svg)"
else
    echo "❌ File heroicons:heart.svg does not exist"
    exit 1
fi

echo "🧹 Cleaning up test directory..."
cd "$OLDPWD"
rm -rf "$TEST_DIR"

echo "✅ All tests passed! The command works as expected."

# Expected behavior:
# - Creates folder: src/assets/icons/
# - Creates file: src/assets/icons/index.ts with content:
#   export { default as IconHeart } from './heroicons:heart.svg';
# - Creates file: src/assets/icons/heroicons:heart.svg with valid SVG content
