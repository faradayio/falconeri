# This is a `justfile`, which is sort of like a less crufty makefile.
# It's processed using https://github.com/casey/just, which you can
# install using `cargo install -f just`.
#
# To see a list of available commands, run `just --list`.

# This should be either "debug" or "release". You can pass `mode=release` on
# the command line to perform a release build.
mode = "debug"

# The docker image `build-falconeri`, which we use to compile things.
_build_falconeri_image:
    docker build -f Dockerfile.build -t build-falconeri .

# The container `build-falconeri-run`, which contains our binaries and docs.
#
# This uses a bash script so it can get access to more features.
_build_falconeri_container: _build_falconeri_image
    #!/usr/bin/env bash
    set -euo pipefail
    docker rm build-falconeri-container || true
    if [ "{{mode}}" == debug ]; then
        docker run \
            -v falconeri-cargo-git:/home/rust/.cargo/git \
            -v falconeri-cargo-git:/home/rust/.cargo/registry \
            -v falconeri-target:/home/rust/src/target \
            --name build-falconeri-container \
            build-falconeri
    else
        docker run \
            -e CARGO_ARGS=--release \
            --name build-falconeri-container \
            build-falconeri
    fi

# Create a `bin/{{mode}}/` directory with our various binaries.
static-bin: _build_falconeri_container
    mkdir -p 'bin/{{mode}}'
    docker cp 'build-falconeri-container:/home/rust/src/target/x86_64-unknown-linux-musl/{{mode}}/falconeri' 'bin/{{mode}}/falconeri'
    docker cp 'build-falconeri-container:/home/rust/src/target/x86_64-unknown-linux-musl/{{mode}}/falconerid' 'bin/{{mode}}/falconerid'
    docker cp 'build-falconeri-container:/home/rust/src/target/x86_64-unknown-linux-musl/{{mode}}/falconeri-worker' 'bin/{{mode}}/falconeri-worker'

# Create a `gh-pages` directory with our "GitHub pages" documentation.
gh-pages: _build_falconeri_container
    rm -rf gh-pages
    docker cp build-falconeri-container:/home/rust/src/guide/book gh-pages

# Our `falconeri` Docker image.
image: static-bin
    docker build --build-arg MODE={{mode}} -t falconeri .
