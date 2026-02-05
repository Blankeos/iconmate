set dotenv-load := true

default:
    just --list

# Usage: just config-schema
config-schema:
    cd config-gen && bun install && bun run generate

# Tag and release a new version - custom script by carlo.

# Usage: just tag_and_release
tag:
    sh tag_and_release.sh
