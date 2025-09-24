#!/usr/bin/env bash
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

echo "🦋 Updating Cargo.toml to version ${NEW}"
sed -i.bak "s/^version *= *\"[^\"]*\"/version = \"${NEW}\"/" Cargo.toml
rm Cargo.toml.bak

echo "🦋 Creating git tag v${NEW}"
git tag "v${NEW}"


echo "🦋 Pushing..."
# git push --tags
