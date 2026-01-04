# This is a `justfile`, which is sort of like a less crufty makefile.
# It's processed using https://github.com/casey/just, which you can
# install using `cargo install -f just`.
#
# To see a list of available commands, run `just --list`.
#
# To make an alpha release:
#
# 1. Run `just set-version 0.x.y-alpha.z`, where `0.x.y` will be the next
#    release.
# 2. Run `just publish-image`.
# 3. Run `cargo run -p falconeri -- deploy` to update `falconerid`.

# This should be either "debug" or "release". You can pass `mode=release` on
# the command line to perform a release build.
MODE := "debug"

# Look up our CLI version (which should match our other package versions).
VERSION := `cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "falconeri") | .version'`

# Print the current version.
version:
    @echo "{{VERSION}}"

# Update all versions. Usage:
#
#     just set-version 0.2.1
#
# TEMPORARY: This will have to be improved before we can make crate releases,
# because it doesn't update inter-crate dependencies. We need something like
# this. See https://github.com/killercup/cargo-edit/issues/426.
set-version NEW_VERSION:
    # If this fails, run `cargo install cargo-edit`.
    cargo set-version --workspace {{NEW_VERSION}}

# Build all binaries and create a `bin/{{MODE}}/` directory.
bin:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "{{MODE}}" == "release" ]; then
        cargo build --all --release
    else
        cargo build --all
    fi
    mkdir -p 'bin/{{MODE}}'
    cp 'target/{{MODE}}/falconeri' 'bin/{{MODE}}/falconeri'
    cp 'target/{{MODE}}/falconerid' 'bin/{{MODE}}/falconerid'
    cp 'target/{{MODE}}/falconeri-worker' 'bin/{{MODE}}/falconeri-worker'

# Create a `gh-pages` directory with our "GitHub pages" documentation.
gh-pages:
    cd guide && mdbook build
    rm -rf gh-pages
    cp -r guide/book gh-pages

# Our `falconeri` Docker image.
image: bin
    docker build --build-arg MODE={{MODE}} -t faraday/falconeri:{{VERSION}} .

# This will publish our image to Docker Hub. Obviously, this requires an
# authorized account.
#
# Before doing this, update version in _all_ Cargo.toml files to a new version.
publish-image: image
    docker push faraday/falconeri:{{VERSION}}

# Check to make sure that we're in releasable shape.
check:
    cargo fmt -- --check
    cargo deny check
    cargo clippy -- -D warnings
    cargo test --all

# Check to make sure our working copy is clean.
check-clean:
    git diff-index --quiet HEAD --

# PLEASE DO NOT RUN WITHOUT SIGN-OFF FROM emk. This is not a complete set of
# things that need to be done for a valid release. Some other things:
#
# 1. The top-most commit must be a valid release commit in a consistent
#    format.
# 2. We must follow an as-yet-incomplete semver policy.
#
# If you need to make an internal testing release, you should instead:
#
#     just set-version x.y.z-alpha.n
#     just publish-image
#
# Call this as:
#
#     just MODE=release release
#
release: check check-clean publish-image
    git tag v{{VERSION}}
    git push
    git push --tags
