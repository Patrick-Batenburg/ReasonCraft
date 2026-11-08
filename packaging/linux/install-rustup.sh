#!/usr/bin/env bash
# Installs rustup on Linux without piping a download into a shell: fetches rustup-init of the
# pinned version for this machine's architecture, checks it against the pinned SHA-256, and runs it
# with no default toolchain (the caller then runs `rustup toolchain install`, which reads
# rust-toolchain.toml). Used by packaging/linux/Dockerfile and the release workflow's arm64 Linux
# runner, which has no rustup.
#
# Usage: packaging/linux/install-rustup.sh
# Env: RUSTUP_HOME and CARGO_HOME as rustup reads them (default ~/.rustup and ~/.cargo).
#
# To upgrade: take the version from https://static.rust-lang.org/rustup/release-stable.toml and
# each hash from https://static.rust-lang.org/rustup/archive/<version>/<target>/rustup-init.sha256.
set -euo pipefail

RUSTUP_VERSION=1.29.2
case "$(uname -m)" in
  x86_64)
    target=x86_64-unknown-linux-gnu
    sha256=5be191e8c70a5007074254cb74d78dbfdd25cd5a61c27de7b8e5410122ec24f4
    ;;
  aarch64 | arm64)
    target=aarch64-unknown-linux-gnu
    sha256=a9c168dbea81a84671c6b7a5c1795cb0eb699020110d107ddfca427bd797e697
    ;;
  *) echo "error: no pinned rustup-init for $(uname -m)" >&2; exit 1 ;;
esac

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
curl --proto '=https' --tlsv1.2 -fsSL -o "$work/rustup-init" \
  "https://static.rust-lang.org/rustup/archive/$RUSTUP_VERSION/$target/rustup-init"
printf '%s  %s\n' "$sha256" "$work/rustup-init" | sha256sum -c -
chmod 755 "$work/rustup-init"
# In a container job the runner sets HOME=/github/home while running as root, whose home is
# /root: rustup-init then takes it for sudo and prints errors, though it goes on.
RUSTUP_INIT_SKIP_SUDO_CHECK=yes "$work/rustup-init" -y --no-modify-path --profile minimal --default-toolchain none
