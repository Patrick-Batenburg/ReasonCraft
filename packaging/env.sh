# shellcheck shell=bash
# Adapted from WordCraft (https://github.com/storytold/wordcraft, packaging/env.sh at aadb077),
# MIT OR Apache-2.0, used under Apache-2.0; changed: REASONCRAFT_VERSION, CARGO_BUILD_JOBS, no
# build SHA or date, no craft-fonts licences, and copy_docs takes ReasonCraft's documents.
#
# Shared setup for the packaging scripts. Source it: `. "$(dirname "$0")/../env.sh"`.
#
# Exports:
#   ROOT              workspace root
#   VERSION           [workspace.package] version from Cargo.toml (override: REASONCRAFT_VERSION)
#   DIST              output directory for release artifacts (default: $ROOT/dist/release)
#   CARGO_TARGET_DIR  cargo's target dir (default: $ROOT/target)
#   CARGO_BUILD_JOBS  parallel cargo jobs (default: 4; each link needs a few GB of memory)
#
# CRAFT_FONTS_DIR and CRAFT_I18N_DIR are ignored: ReasonCraft bundles its own fonts and i18n.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export ROOT

# The version lives in exactly one place: `[workspace.package] version` in the root Cargo.toml.
# (`cargo xtask version` prints the same thing; awk avoids compiling xtask here.)
workspace_version() {
  awk '
    { sub(/\r$/, "") }
    /^\[/ { in_pkg = ($0 == "[workspace.package]"); next }
    in_pkg && $1 == "version" { gsub(/[" ]/, "", $3); print $3; exit }
  ' "$ROOT/Cargo.toml"
  return $?
}

VERSION="${REASONCRAFT_VERSION:-$(workspace_version)}"
if [[ -z "$VERSION" ]]; then
  echo "error: could not read [workspace.package] version from $ROOT/Cargo.toml" >&2
  exit 1
fi
export VERSION

DIST="${DIST:-$ROOT/dist/release}"
mkdir -p "$DIST"
export DIST

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"

# Emit a GitHub Actions warning (plain stderr outside Actions).
warn() {
  if [[ -n "${GITHUB_ACTIONS:-}" ]]; then echo "::warning::$*"; else echo "warning: $*" >&2; fi
  return 0
}

# Signing settings are all there or not at all (ruling PK-16). Usage: signing_set_complete <what>
# <VAR>...; returns 0 when every VAR is set or none is. When only some are, it names the missing
# ones (never a value) in a warning, or, with REASONCRAFT_REFUSE_PARTIAL_SIGNING=1 (the release
# workflow on the release branch), in an error and returns 1.
signing_set_complete() {
  local what="$1" name
  shift
  local set=() missing=()
  for name in "$@"; do
    if [[ -n "${!name:-}" ]]; then set+=("$name"); else missing+=("$name"); fi
  done
  if [[ ${#set[@]} -eq 0 || ${#missing[@]} -eq 0 ]]; then
    return 0
  fi
  local message="$what is partly configured: ${set[*]} set, ${missing[*]} missing"
  if [[ "${REASONCRAFT_REFUSE_PARTIAL_SIGNING:-}" == 1 ]]; then
    echo "error: $message; set all of them or none before releasing" >&2
    summary "- $what: partly configured (${missing[*]} missing), the build failed"
    return 1
  fi
  warn "$message"
  return 0
}

# Append a line to the GitHub Actions job summary (nothing outside Actions).
summary() {
  if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then echo "$*" >>"$GITHUB_STEP_SUMMARY"; fi
  return 0
}

# Copy the readme, the licences and the third-party notices into a package directory.
copy_docs() {
  local dest="$1" f
  for f in README.md LICENSE-MIT LICENSE-APACHE THIRD-PARTY.md; do
    cp "$ROOT/$f" "$dest/"
  done
  return 0
}

# Portable SHA-256 of a file (prints just the hash).
sha256() {
  local file="$1"
  if command -v sha256sum >/dev/null; then sha256sum "$file" | cut -d' ' -f1; else shasum -a 256 "$file" | cut -d' ' -f1; fi
  return $?
}
