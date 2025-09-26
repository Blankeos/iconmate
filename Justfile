set dotenv-load

default:
    just --list

# Tag and release a new version - custom script by carlo.
# Usage: just tag_and_release
tag:
    sh tag_and_release.sh
