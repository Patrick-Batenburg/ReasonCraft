#!/usr/bin/env bash
# Run the app of a DMG as users get it: `ReasonCraft --version` must print `ReasonCraft <version>`.
#
# Usage: packaging/macos/run-from-dmg.sh <dmg> <version>
#
# Mounts the DMG read-only and runs the app in it on a throw-away profile (REASONCRAFT_PROFILE_DIR),
# stopped after a minute should it ever open its window instead (macOS has no `timeout`), so it
# never touches the real settings. package.sh runs it on the DMG it made; the release workflow runs
# it in a job of its own, apart from the signing secrets.
set -euo pipefail

if [[ $# -ne 2 ]]; then
  sed -n '4p' "$0" >&2
  exit 2
fi
DMG="$1"
VERSION="$2"

WORK="$(mktemp -d)"
MOUNT="$WORK/mount"
mkdir -p "$MOUNT"
cleanup() {
  hdiutil detach "$MOUNT" >/dev/null 2>&1 || hdiutil detach "$MOUNT" -force >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

# On CI runners hdiutil attach fails now and then with "Resource busy": a few tries.
attached=0
for try in 1 2 3 4 5; do
  if hdiutil attach -readonly -nobrowse -noautoopen -mountpoint "$MOUNT" "$DMG" >/dev/null; then
    attached=1
    break
  fi
  echo "hdiutil attach failed (try $try of 5); retrying in $((try * 10)) s" >&2
  sleep $((try * 10))
done
if [[ "$attached" = 0 ]]; then
  echo "error: could not mount $(basename "$DMG")" >&2
  exit 1
fi

EXE="$MOUNT/ReasonCraft.app/Contents/MacOS/ReasonCraft"
if [[ ! -x "$EXE" ]]; then
  echo "error: $(basename "$DMG") has no runnable ReasonCraft.app/Contents/MacOS/ReasonCraft" >&2
  exit 1
fi
mkdir -p "$WORK/profile"
REASONCRAFT_PROFILE_DIR="$WORK/profile" "$EXE" --version </dev/null >"$WORK/out.txt" 2>"$WORK/err.txt" &
pid=$!
for _ in $(seq 60); do
  kill -0 "$pid" 2>/dev/null || break
  sleep 1
done
if kill -0 "$pid" 2>/dev/null; then
  kill "$pid"
  echo "error: ReasonCraft --version did not exit within a minute (it opened the window?)" >&2
  exit 1
fi
status=0
wait "$pid" || status=$?
line="$(tr -d '\r' <"$WORK/out.txt" | head -n 1)"
if [[ "$status" != 0 ]]; then
  echo "error: ReasonCraft --version exited $status: $line $(cat "$WORK/err.txt")" >&2
  exit 1
fi
if [[ "$line" != "ReasonCraft $VERSION" ]]; then
  echo "error: ReasonCraft --version printed '$line', expected 'ReasonCraft $VERSION'" >&2
  exit 1
fi
echo "PASS $(basename "$DMG"): $line"
