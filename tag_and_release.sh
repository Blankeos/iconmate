#!/usr/bin/env bash

# THIS SCRIPT IS CUSTOM - inspired from changesets.
# The difference is, there is no workflow. So everything runs from your computer.
# Which also means, no collaboration kind of, not everyone can release.

set -euo pipefail

if [ -n "$(git status --porcelain)" ]; then
    echo "❗ Please commit all changes before bumping the version."
    exit 1
fi

# Written by AI :)
NAME=$(sed -n 's/^name *= *"\([^"]*\)".*/\1/p' Cargo.toml)
CURRENT=$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' Cargo.toml)
echo "🦋 What kind of change is this for $NAME? (current version is $CURRENT) [patch, minor, major] >"

read -r BUMP

case "$BUMP" in
    patch) NEW=$(echo "$CURRENT" | awk -F. '{$NF+=1; OFS="."; print $1,$2,$3}') ;;
    minor) NEW=$(echo "$CURRENT" | awk -F. '{$(NF-1)+=1; $NF=0; OFS="."; print $1,$2,$3}') ;;
    major) NEW=$(echo "$CURRENT" | awk -F. '{$1+=1; $2=0; $3=0; OFS="."; print $1,$2,$3}') ;;
    *) echo "Please specify patch, minor, or major"; exit 1 ;;
esac

echo "🦋 Would tag and push $NAME $CURRENT -> $NEW"

read -p "Proceed? [Y/n] " -r CONFIRM
CONFIRM=${CONFIRM:-y}
if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

# ============================================
# Update & Commit - Release manifests
# ============================================

# Update the Cargo.toml
echo "🦋 Updating Cargo.toml to version ${NEW}"
sed -i.bak "s/^version *= *\"[^\"]*\"/version = \"${NEW}\"/" Cargo.toml
rm Cargo.toml.bak

# Update npm/package.json if it exists
if [ -f "npm/package.json" ]; then
    echo "🦋 Updating npm/package.json to version ${NEW}"
    sed -i.bak "s/\"version\":[[:space:]]*\"[^\"]*\"/\"version\": \"${NEW}\"/" npm/package.json
    rm npm/package.json.bak
    git add npm/package.json
fi

# Commit
echo "🦋 Committing version bump ${NEW}..."
sleep 2 # wait a 2 secs because Cargo.lock will get updated
git add .
sleep 2lazyg # wait a 2 secs because Cargo.lock will get updated
git commit -m "release: ${NAME} v${NEW}"

# ============================================
# cargo-dist Publish GitHub Releases via actions
# ============================================

# Create the git tag.
echo "🦋 Creating git tag v${NEW}"
git tag "v${NEW}"

# Create release binaries (with cargo-dist)
echo "🦋 Pushing..."
git push --tags
git push

# ============================================
# PUBLISHING: I put it here as documentation, but this is manual for now!
# ============================================

# crates.io
# cargo publish

# npm
# cd npm
# npm publish
