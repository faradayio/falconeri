FROM ekidd/rust-musl-builder:nightly-2019-04-12

# We need to add the source code to the image because `rust-musl-builder`
# assumes a UID of 1000, but TravisCI has switched to 2000.
ADD . ./
RUN sudo chown -R rust:rust .

# Build all binaries when on Linux.
CMD cargo build --all --release && cd guide && mdbook build
