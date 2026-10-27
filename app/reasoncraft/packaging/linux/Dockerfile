# Linux build image: runs the test suite and builds the Linux installers
# (see docs/release.md). Build it from the repository root, so that the
# toolchain pinned in rust-toolchain.toml is installed into the image:
#
#   docker build -f app/reasoncraft/packaging/linux/Dockerfile -t reasoncraft-linux .
#
# Dockerfile.dockerignore keeps the build context down to that one file.
FROM rust:1.99-bookworm

# egui's system libraries (winit, the GTK file dialogs of rfd, arboard).
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        libxkbcommon-dev libgtk-3-dev \
        libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
        libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# The pinned toolchain, with clippy and rustfmt.
COPY rust-toolchain.toml /tmp/toolchain/
RUN cd /tmp/toolchain && rustup toolchain install \
    && rm -rf /tmp/toolchain

RUN cargo install cargo-packager --version 0.11.8 --locked

WORKDIR /src
