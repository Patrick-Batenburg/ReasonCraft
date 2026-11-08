#!/usr/bin/env bash
# Adapted from WordCraft (https://github.com/storytold/wordcraft, packaging/linux/package.sh at
# aadb077), used under MIT OR Apache-2.0; changed: one binary (no CLI), no
# cross-compiling, icons staged from app/reasoncraft/assets, REASONCRAFT_MAINTAINER, update
# information and .zsync only for builds in GitHub Actions, and the --version smoke run on a
# throw-away profile.
#
# Build and package ReasonCraft for Linux (<arch> is x86_64 or aarch64, the host's):
#
#   $DIST/reasoncraft-<version>-linux-<arch>.AppImage        any distro with glibc >= the build host's
#   $DIST/reasoncraft-<version>-linux-<arch>.AppImage.zsync  delta updates (GITHUB_REPOSITORY set)
#   $DIST/reasoncraft-<version>-linux-<arch>.deb             Debian, Ubuntu, Mint, Pop!_OS, ...
#   $DIST/reasoncraft-<version>-linux-<arch>.rpm             Fedora, openSUSE, RHEL, ...
#   $DIST/reasoncraft-<version>-linux-<arch>.tar.gz          plain FHS-style tree (bin/, share/)
#
# Usage: packaging/linux/package.sh [--skip-build] [--formats "appimage deb rpm tar"]
#
# Needs: cargo; nfpm for deb/rpm (https://nfpm.goreleaser.com); appimagetool for the AppImage
# (downloaded into $CARGO_TARGET_DIR if missing, checked against its SHA-256). Build on an old
# distro (packaging/linux/Dockerfile and CI: Ubuntu 22.04, glibc 2.35) so the binary runs on newer
# ones. Optional: desktop-file-validate, appstreamcli, zsyncmake (the zsync package).
set -euo pipefail
# shellcheck source=../env.sh
. "$(dirname "${BASH_SOURCE[0]}")/../env.sh"
HERE="$ROOT/packaging/linux"
ASSETS="$ROOT/app/reasoncraft/assets"
APP_ID=io.github.Patrick_Batenburg.ReasonCraft

SKIP_BUILD=0
FORMATS="appimage deb rpm tar"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build) SKIP_BUILD=1; shift ;;
    --formats) FORMATS="$2"; shift 2 ;;
    -h | --help) sed -n '8,21p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) DEB_ARCH=amd64 ;;
  aarch64 | arm64) ARCH=aarch64; DEB_ARCH=arm64 ;;
  *) echo "unsupported architecture $ARCH" >&2; exit 2 ;;
esac
export REASONCRAFT_MAINTAINER="${REASONCRAFT_MAINTAINER:-ReasonCraft contributors}"
BASENAME="reasoncraft-$VERSION-linux-$ARCH"
DATE="$(date -u +%Y-%m-%d)"

echo "==> ReasonCraft $VERSION for Linux $ARCH ($FORMATS)"

if [[ "$SKIP_BUILD" = 0 ]]; then
  (cd "$ROOT" && cargo build --release --locked -p reasoncraft)
fi
BIN="$CARGO_TARGET_DIR/release"
WORK="$CARGO_TARGET_DIR/linux-package"
STAGE="$WORK/root"
rm -rf "$WORK"

# ---- stage an FHS tree (shared by every format) -------------------------------------------------
install -Dm755 "$BIN/reasoncraft" "$STAGE/usr/bin/reasoncraft"
strip "$STAGE/usr/bin/reasoncraft"
install -Dm644 "$HERE/$APP_ID.desktop" "$STAGE/usr/share/applications/$APP_ID.desktop"
install -Dm644 "$HERE/$APP_ID.mime.xml" "$STAGE/usr/share/mime/packages/$APP_ID.xml"
mkdir -p "$STAGE/usr/share/metainfo"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@DATE@/$DATE/g" \
  "$HERE/$APP_ID.metainfo.xml.in" >"$STAGE/usr/share/metainfo/$APP_ID.metainfo.xml"
for s in 16 24 32 48 64 128 256 512; do
  install -Dm644 "$ASSETS/icons/icon-$s.png" "$STAGE/usr/share/icons/hicolor/${s}x${s}/apps/$APP_ID.png"
done
install -Dm644 "$ASSETS/icon.svg" "$STAGE/usr/share/icons/hicolor/scalable/apps/$APP_ID.svg"
mkdir -p "$STAGE/usr/share/doc/reasoncraft"
copy_docs "$STAGE/usr/share/doc/reasoncraft"

if command -v desktop-file-validate >/dev/null; then
  desktop-file-validate "$STAGE/usr/share/applications/$APP_ID.desktop"
fi
if command -v appstreamcli >/dev/null; then
  appstreamcli validate --no-net --explain "$STAGE/usr/share/metainfo/$APP_ID.metainfo.xml"
fi

has() { local item="$1"; case " $FORMATS " in *" $item "*) return 0 ;; *) return 1 ;; esac; }

# ---- .tar.gz ------------------------------------------------------------------------------------
if has tar; then
  mkdir -p "$WORK/tar"
  cp -R "$STAGE/usr" "$WORK/tar/$BASENAME"
  # Owned by root and writable only by the owner in the archive, whatever the build user and its
  # umask: unpacked as root, the files would otherwise belong to whoever has the build user's uid
  # (1001 on GitHub's runners), with group-writable folders.
  tar -C "$WORK/tar" --owner=0 --group=0 --numeric-owner --mode=u+rwX,go+rX,go-w \
    -czf "$DIST/$BASENAME.tar.gz" "$BASENAME"
  echo "wrote $DIST/$BASENAME.tar.gz"
fi

# ---- .deb / .rpm --------------------------------------------------------------------------------
if has deb || has rpm; then
  command -v nfpm >/dev/null || { echo "error: nfpm not found (https://nfpm.goreleaser.com/install/)" >&2; exit 1; }
  export NFPM_ARCH="$DEB_ARCH"
  # nfpm expands env vars in fields like `version` and `arch`, but not in `contents[].src`.
  sed "s|\${STAGE}|$STAGE|g" "$HERE/nfpm.yaml" >"$WORK/nfpm.yaml"
  for fmt in deb rpm; do
    if has "$fmt"; then (cd "$ROOT" && nfpm package -f "$WORK/nfpm.yaml" -p "$fmt" -t "$DIST/$BASENAME.$fmt"); fi
  done
fi

# ---- AppImage -----------------------------------------------------------------------------------
if has appimage; then
  APPIMAGETOOL_VERSION=1.9.1
  # appimagetool would otherwise embed type2-runtime's `continuous` build, unchecked.
  RUNTIME_VERSION=20251108
  # From the GitHub release assets' SHA-256 digests (checked again on 2026-10-28).
  case "$ARCH" in
    x86_64)
      APPIMAGETOOL_SHA256=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
      RUNTIME_SHA256=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d
      ;;
    aarch64)
      APPIMAGETOOL_SHA256=f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158
      RUNTIME_SHA256=00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444
      ;;
  esac
  # Downloads $1 to $3 once and checks it against SHA-256 $2, the cached copy again on every run.
  fetch_pinned() {
    local url="$1" expected_sha="$2" target="$3"
    if [[ ! -x "$target" ]]; then
      curl --proto '=https' -fsSL -o "$target.download" "$url"
      printf '%s  %s\n' "$expected_sha" "$target.download" | sha256sum -c -
      chmod 755 "$target.download"
      mv "$target.download" "$target"
    fi
    printf '%s  %s\n' "$expected_sha" "$target" | sha256sum -c -
    return 0
  }
  APPDIR="$WORK/ReasonCraft.AppDir"
  cp -R "$STAGE" "$APPDIR"
  ln -s usr/bin/reasoncraft "$APPDIR/AppRun"
  cp "$HERE/$APP_ID.desktop" "$APPDIR/$APP_ID.desktop"
  cp "$ASSETS/icons/icon-256.png" "$APPDIR/$APP_ID.png"
  ln -s "$APP_ID.png" "$APPDIR/.DirIcon"

  TOOL="${APPIMAGETOOL:-$(command -v appimagetool || true)}"
  if [[ -z "$TOOL" ]]; then
    TOOL="$CARGO_TARGET_DIR/appimagetool-$APPIMAGETOOL_VERSION-$ARCH.AppImage"
    fetch_pinned "https://github.com/AppImage/appimagetool/releases/download/$APPIMAGETOOL_VERSION/appimagetool-$ARCH.AppImage" \
      "$APPIMAGETOOL_SHA256" "$TOOL"
  fi
  RUNTIME="$CARGO_TARGET_DIR/appimage-runtime-$RUNTIME_VERSION-$ARCH"
  fetch_pinned "https://github.com/AppImage/type2-runtime/releases/download/$RUNTIME_VERSION/runtime-$ARCH" \
    "$RUNTIME_SHA256" "$RUNTIME"
  # Absolute, because appimagetool runs in $DIST below (CARGO_TARGET_DIR or APPIMAGETOOL may be
  # relative).
  OUT="$(cd "$DIST" && pwd)/$BASENAME.AppImage"
  APPDIR="$(cd "$APPDIR" && pwd)"
  TOOL="$(cd "$(dirname "$TOOL")" && pwd)/$(basename "$TOOL")"
  RUNTIME="$(cd "$(dirname "$RUNTIME")" && pwd)/$(basename "$RUNTIME")"
  # A .zsync left by an earlier run would hide a missing zsyncmake and describe another file.
  rm -f "$OUT.zsync"
  # Update information: AppImageUpdate and the like read it from the file and fetch only the
  # blocks that changed in a newer release, through the .zsync published next to each AppImage on
  # GitHub Releases (`latest` is the newest published release that is not a pre-release). Only a
  # build in GitHub Actions knows its repository; a local build carries no update information.
  UPDATE=()
  if [[ -n "${GITHUB_REPOSITORY:-}" ]]; then
    REPO="$GITHUB_REPOSITORY"
    UPDATE=(-u "gh-releases-zsync|${REPO%%/*}|${REPO#*/}|latest|reasoncraft-*-linux-$ARCH.AppImage.zsync")
  fi
  # Extract-and-run: works without FUSE (containers, CI). The output embeds the pinned static
  # runtime, so users don't need libfuse2 either. With update information and zsyncmake on the
  # host, appimagetool also writes the .zsync, into its working directory, hence the cd.
  (cd "$DIST" && ARCH="$ARCH" APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" --no-appstream \
    --runtime-file "$RUNTIME" ${UPDATE[@]+"${UPDATE[@]}"} "$APPDIR" "$OUT")
  echo "wrote $OUT"
  if [[ -s "$OUT.zsync" ]]; then
    echo "wrote $OUT.zsync"
  elif [[ -n "${GITHUB_REPOSITORY:-}" ]]; then
    warn "zsyncmake not found, so $OUT.zsync was not written; AppImage delta updates need it"
  fi
fi

# Smoke run. Should `--version` ever open the window, it does so on a throw-away profile and is
# stopped, never touching the real settings.
PROFILE="$(mktemp -d)"
REASONCRAFT_PROFILE_DIR="$PROFILE" timeout 60 "$STAGE/usr/bin/reasoncraft" --version </dev/null
rm -rf "$PROFILE"
echo "==> done"
ls -lh "$DIST"
