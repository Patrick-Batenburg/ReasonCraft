# Linux build image: runs the test suite and builds the Linux packages (see docs/release.md).
# Ubuntu 22.04 (glibc 2.35), as on CI, so the binary runs on that and every newer distro.
# Build it from the repository root, so that the toolchain pinned in rust-toolchain.toml is
# installed into the image:
#
#   docker build -f packaging/linux/Dockerfile -t reasoncraft-linux .
#
# Dockerfile.dockerignore keeps the build context down to that file and install-rustup.sh.
#
# The base image is pinned by the digest of its multi-architecture index (amd64 and arm64); to
# update it: `docker buildx imagetools inspect ubuntu:22.04` and take the top `Digest:`.
FROM ubuntu:22.04@sha256:e0f80c829509ef4df9e76bd8944eb7b8ec25cbb379b112a7f9448b2c77a77c9b

ARG DEBIAN_FRONTEND=noninteractive
# The same packages as the release workflow's Linux job: the C toolchain (no -dev packages: winit,
# wgpu, glow and rfd open the system libraries at run time or talk D-Bus in Rust), curl and git,
# file (appimagetool needs it), the validators package.sh runs (desktop-file-validate,
# appstreamcli), zsyncmake for the AppImage's .zsync, and rpm to inspect the .rpm.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential ca-certificates curl file git \
        desktop-file-utils appstream zsync rpm \
    && rm -rf /var/lib/apt/lists/*

# nfpm for the .deb and .rpm, checked against the SHA-256 in its release's checksums.txt.
ARG NFPM_VERSION=2.47.0
RUN set -eu; \
    case "$(uname -m)" in \
      x86_64) arch=x86_64; sha=0660ca602b2d2d2ae4781a06c692b3eeb9d437ffea05b831d76e41f4a3188783 ;; \
      aarch64) arch=arm64; sha=1c0f5f2999b9a974bfb04fdb0cc3306096de530ac5dbb25d739cc5f5219c919c ;; \
      *) echo "unsupported architecture" >&2; exit 1 ;; \
    esac; \
    file="nfpm_${NFPM_VERSION}_Linux_${arch}.tar.gz"; \
    curl --proto '=https' -fsSL -o "/tmp/$file" \
      "https://github.com/goreleaser/nfpm/releases/download/v${NFPM_VERSION}/$file"; \
    echo "$sha  /tmp/$file" | sha256sum -c -; \
    tar -xzf "/tmp/$file" -C /usr/local/bin nfpm; \
    rm "/tmp/$file"; \
    nfpm --version

# rustup (rustup-init checked against its pinned SHA-256), then the pinned toolchain with clippy
# and rustfmt.
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo PATH=/usr/local/cargo/bin:$PATH
COPY packaging/linux/install-rustup.sh /tmp/
RUN bash /tmp/install-rustup.sh && rm /tmp/install-rustup.sh
COPY rust-toolchain.toml /tmp/toolchain/
RUN cd /tmp/toolchain && rustup toolchain install \
    && rm -rf /tmp/toolchain

WORKDIR /src
