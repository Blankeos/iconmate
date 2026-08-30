set dotenv-load := true

default:
    just --list

# Usage: just config-schema
config-schema:
    cd config-gen && bun install && bun run generate

dev:
    cargo r

dist-build *args:
    dist build {{ args }}

sync_readme:
    cp README.md npm/README.md

# Release: bump versions, create release commit, and create a git tag.
# Must be run from an up-to-date main — the script will refuse other branches.
# Usage: just tag [patch|minor|major]
tag bump="":
    sh scripts/tag_and_release.sh {{ bump }}