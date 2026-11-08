#!/usr/bin/env bash
# Adapted from WordCraft (https://github.com/storytold/wordcraft, packaging/macos/package.sh at
# aadb077), MIT OR Apache-2.0, used under Apache-2.0; changed: one app (no CLI zip), a plain DMG
# (no background or .DS_Store), rustup target add, the icon from app/reasoncraft/assets, and the
# --version smoke run on a throw-away profile with a time limit.
#
# Build, sign and (optionally) notarize the macOS release artifact:
#
#   $DIST/reasoncraft-<version>-macos-<arch>.dmg   ReasonCraft.app on a drag-to-Applications DMG
#
# Usage: packaging/macos/package.sh [--arch universal|aarch64|x86_64] [--skip-build | --build-only]
#
# --build-only builds the binaries and stops; --skip-build then packages that build without
# compiling anything. The release workflow runs them as two steps and imports the certificate
# between them, so that no build script or proc macro runs with the signing secrets around.
#
# Signing (env):
#   MACOS_SIGN_IDENTITY   codesign identity (name or SHA-1). Default "-" = ad hoc (local testing;
#                         Gatekeeper rejects the result on other Macs). In CI, import-cert.sh sets it.
#   MACOS_KEYCHAIN        keychain holding the identity (optional)
# Notarization (env; all three needed, and a real identity):
#   APPLE_ID, APPLE_PASSWORD (app-specific password), APPLE_TEAM_ID
# An identity without all three, or credentials without an identity, warns; with
# REASONCRAFT_REFUSE_PARTIAL_SIGNING=1 (the release branch) it fails (ruling PK-16).
set -euo pipefail
# shellcheck source=../env.sh
. "$(dirname "${BASH_SOURCE[0]}")/../env.sh"
HERE="$ROOT/packaging/macos"

ARCH=universal
SKIP_BUILD=0
BUILD_ONLY=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --arch) ARCH="$2"; shift 2 ;;
    --skip-build) SKIP_BUILD=1; shift ;;
    --build-only) BUILD_ONLY=1; shift ;;
    -h | --help) sed -n '7,22p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
case "$ARCH" in
  universal) TARGETS=(aarch64-apple-darwin x86_64-apple-darwin) ;;
  aarch64) TARGETS=(aarch64-apple-darwin) ;;
  x86_64) TARGETS=(x86_64-apple-darwin) ;;
  *) echo "unknown --arch $ARCH" >&2; exit 2 ;;
esac
if [[ "$SKIP_BUILD" = 1 && "$BUILD_ONLY" = 1 ]]; then
  echo "--skip-build and --build-only exclude each other" >&2
  exit 2
fi

# Keep in sync with LSMinimumSystemVersion in Info.plist.in (a test checks it).
export MACOSX_DEPLOYMENT_TARGET=11.0
SHORT_VERSION="${VERSION%%-*}"
WORK="$CARGO_TARGET_DIR/macos-package"
APP="$WORK/ReasonCraft.app"
DMG="$DIST/reasoncraft-$VERSION-macos-$ARCH.dmg"

# The signing settings, checked before the build, so that a partly configured set fails at once.
if [[ "$BUILD_ONLY" = 0 ]]; then
  # "-" (ad hoc) counts as no identity.
  if [[ "${MACOS_SIGN_IDENTITY:-}" == "-" ]]; then unset MACOS_SIGN_IDENTITY; fi
  signing_set_complete "macOS signing" MACOS_SIGN_IDENTITY APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID || exit 1
  IDENTITY="${MACOS_SIGN_IDENTITY:--}"

  NOTARIZE=0
  if [[ "$IDENTITY" = "-" ]]; then
    warn "macOS: ad hoc signing (no MACOS_SIGN_IDENTITY); the DMG is not notarized"
    # How users open it: README.md, "Install".
    OUTCOME="**ad hoc signed, not notarized** (no signing secrets; Gatekeeper blocks it on other Macs until the user allows it: on macOS 15 and later System Settings → Privacy & Security → Open Anyway, on macOS 11 to 14 right-click → Open)"
  elif [[ -n "${APPLE_ID:-}" ]] && [[ -n "${APPLE_PASSWORD:-}" ]] && [[ -n "${APPLE_TEAM_ID:-}" ]]; then
    NOTARIZE=1
    OUTCOME="Developer ID signed (hardened runtime), notarized, ticket stapled to the app and the DMG"
  else
    warn "macOS: APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID incomplete; signed but not notarized"
    OUTCOME="Developer ID signed, **not notarized** (notarization credentials incomplete)"
  fi
fi

# ---- build -------------------------------------------------------------------------------------
if [[ "$SKIP_BUILD" = 0 ]]; then
  echo "==> building ReasonCraft $VERSION for macOS ($ARCH)"
  args=()
  for t in "${TARGETS[@]}"; do
    # A caller may only have installed targets for `stable`, not for the pinned toolchain.
    (cd "$ROOT" && rustup target add "$t")
    args+=(--target "$t")
  done
  (cd "$ROOT" && cargo build --release --locked -p reasoncraft "${args[@]}")
fi
if [[ "$BUILD_ONLY" = 1 ]]; then
  echo "==> built; package it with --skip-build"
  exit 0
fi
echo "==> packaging ReasonCraft $VERSION for macOS ($ARCH), identity: $IDENTITY, notarize: $NOTARIZE"

rm -rf "$WORK"
mkdir -p "$WORK/bin"
inputs=()
for t in "${TARGETS[@]}"; do inputs+=("$CARGO_TARGET_DIR/$t/release/reasoncraft"); done
lipo -create -output "$WORK/bin/reasoncraft" "${inputs[@]}"
lipo -info "$WORK/bin/reasoncraft"

# ---- signing helpers ---------------------------------------------------------------------------
sign() {
  # Hardened runtime + secure timestamp for a real identity; ad hoc can't be timestamped.
  local ts=(--timestamp)
  [[ "$IDENTITY" = "-" ]] && ts=(--timestamp=none)
  local kc=()
  [[ -n "${MACOS_KEYCHAIN:-}" ]] && kc=(--keychain "$MACOS_KEYCHAIN")
  codesign --force --sign "$IDENTITY" ${kc[@]+"${kc[@]}"} "${ts[@]}" "$@"
}

# `stapler staple` and `stapler validate` both fetch the ticket from Apple's CloudKit, and a dropped
# connection there (NSURLErrorDomain -1005, "Error 68") fails them although notarization worked.
# Both are idempotent, so retry with a pause; a missing or bad ticket still fails every try.
stapler_retry() {
  local try
  for try in 1 2 3 4; do
    xcrun stapler "$1" "$2" && return 0
    [[ "$try" = 4 ]] && break
    echo "stapler $1 failed (try $try of 4); retrying in $((try * 20)) s" >&2
    sleep $((try * 20))
  done
  echo "error: stapler $1 $(basename "$2") failed 4 times" >&2
  exit 1
}

notarize() {
  local file="$1" out id status
  echo "==> notarizing $(basename "$file") (this can take a few minutes)"
  out="$(xcrun notarytool submit "$file" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" --wait --timeout 1h --output-format json)" || true
  echo "$out"
  id="$(printf '%s' "$out" | plutil -extract id raw -o - - 2>/dev/null || true)"
  status="$(printf '%s' "$out" | plutil -extract status raw -o - - 2>/dev/null || true)"
  if [[ "$status" != "Accepted" ]]; then
    if [[ -n "$id" ]]; then
      xcrun notarytool log "$id" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" || true
    fi
    echo "error: notarization of $(basename "$file") failed (status: ${status:-unknown})" >&2
    exit 1
  fi
}

# ---- ReasonCraft.app ---------------------------------------------------------------------------
echo "==> assembling $APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
# Executable and icon carry the display name (CFBundleExecutable / CFBundleIconFile).
cp "$WORK/bin/reasoncraft" "$APP/Contents/MacOS/ReasonCraft"
cp "$ROOT/app/reasoncraft/assets/icon.icns" "$APP/Contents/Resources/ReasonCraft.icns"
copy_docs "$APP/Contents/Resources"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@SHORT_VERSION@/$SHORT_VERSION/g" \
  "$HERE/Info.plist.in" >"$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist"
printf 'APPL????' >"$APP/Contents/PkgInfo"

# codesign --strict rejects extended attributes such as com.apple.FinderInfo or a resource fork
# ("Finder information, or similar detritus not allowed"); a source file copied in can carry them.
# Clear them before signing; nothing after this point may write xattrs onto the bundle.
xattr -cr "$APP"
# Sign inside-out: nested code first, then the bundle itself (no --deep on the final signature).
# Today the only nested code is the main executable; frameworks or helpers would be signed here too.
sign --options runtime --entitlements "$HERE/entitlements.plist" "$APP/Contents/MacOS/ReasonCraft"
sign --options runtime --entitlements "$HERE/entitlements.plist" "$APP"
codesign --verify --strict --deep --verbose=2 "$APP"

if [[ "$NOTARIZE" = 1 ]]; then
  ditto -c -k --keepParent "$APP" "$WORK/ReasonCraft-notarize.zip"
  notarize "$WORK/ReasonCraft-notarize.zip"
  stapler_retry staple "$APP"
  stapler_retry validate "$APP"
  spctl --assess --type execute -vvv "$APP"
fi

# ---- DMG ---------------------------------------------------------------------------------------
echo "==> building $DMG"
STAGE="$WORK/dmg"
mkdir -p "$STAGE"
ditto "$APP" "$STAGE/ReasonCraft.app"
ln -s /Applications "$STAGE/Applications"
rm -f "$DMG"
# Retries an hdiutil verb that attaches a device: on CI runners it fails now and then with
# "Resource busy". Usage: hdiutil_retry <verb> <args...>; five tries with growing pauses.
hdiutil_retry() {
  local verb="$1"
  local attempt
  for attempt in 1 2 3 4 5; do
    if hdiutil "$@"; then
      return 0
    fi
    if [[ "$attempt" = 5 ]]; then
      echo "error: hdiutil $verb failed 5 times" >&2
      return 1
    fi
    warn "hdiutil $verb failed (attempt $attempt of 5); retrying in $((attempt * 10))s"
    sleep $((attempt * 10))
  done
  return 1
}
# `hdiutil create -srcfolder` copies the staged files as they are. `makehybrid -hfs` + `convert`
# would give every file and folder in the image a com.apple.FinderInfo, so the signed app would
# fail `codesign --verify --deep --strict` inside the mounted DMG and after copying it out.
hdiutil_retry create -srcfolder "$STAGE" -volname "ReasonCraft" -fs HFS+ -format UDZO \
  -imagekey zlib-level=9 -ov "$DMG"
sign "$DMG"
codesign --verify --strict --verbose=2 "$DMG"

# Gate: the app inside the finished image, and a copy installed from it with ditto, must pass the
# same strict check as the app before packaging, so a container signature can't hide a damaged app
# signature. Returns non-zero (set -e then stops the build) after detaching the image.
verify_dmg_app() {
  local dmg="$1" mount="$WORK/dmg-check" copy="$WORK/dmg-check-copy" rc=0
  mkdir -p "$mount" "$copy"
  hdiutil_retry attach -readonly -nobrowse -noautoopen -mountpoint "$mount" "$dmg" >/dev/null
  echo "==> strict signature check of ReasonCraft.app inside $(basename "$dmg")"
  codesign --verify --deep --strict --verbose=2 "$mount/ReasonCraft.app" || rc=1
  if [[ "$rc" = 0 ]]; then
    ditto "$mount/ReasonCraft.app" "$copy/ReasonCraft.app" || rc=1
  fi
  if [[ "$rc" = 0 ]]; then
    echo "==> strict signature check of a ditto copy of it"
    codesign --verify --deep --strict --verbose=2 "$copy/ReasonCraft.app" || rc=1
  fi
  if [[ "$rc" = 0 ]] && [[ "$NOTARIZE" = 1 ]]; then
    spctl --assess --type execute -vvv "$copy/ReasonCraft.app" || rc=1
  fi
  if [[ "$rc" != 0 ]]; then
    echo "error: ReasonCraft.app in $(basename "$dmg") fails strict verification; extended attributes:" >&2
    xattr -lr "$mount/ReasonCraft.app" >&2 || true
  fi
  hdiutil_retry detach "$mount" >/dev/null || hdiutil detach "$mount" -force >/dev/null || rc=1
  rm -rf "$copy"
  return "$rc"
}
verify_dmg_app "$DMG"
if [[ "$NOTARIZE" = 1 ]]; then
  notarize "$DMG"
  stapler_retry staple "$DMG"
  stapler_retry validate "$DMG"
  spctl --assess --type open --context context:primary-signature -vvv "$DMG"
fi

# Smoke run. Should `--version` ever open the window, it does so on a throw-away profile and is
# stopped after a minute (macOS has no `timeout`), never touching the real settings.
PROFILE="$(mktemp -d)"
REASONCRAFT_PROFILE_DIR="$PROFILE" "$APP/Contents/MacOS/ReasonCraft" --version </dev/null &
pid=$!
for _ in $(seq 60); do
  kill -0 "$pid" 2>/dev/null || break
  sleep 1
done
if kill -0 "$pid" 2>/dev/null; then
  kill "$pid"
  echo "error: ReasonCraft --version did not exit (it opened the window?)" >&2
  exit 1
fi
wait "$pid"
rm -rf "$PROFILE"
summary "- macOS \`$(basename "$DMG")\`: $OUTCOME"
echo "==> done"
ls -lh "$DMG"
