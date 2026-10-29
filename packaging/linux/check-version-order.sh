#!/usr/bin/env bash
# Checks that nfpm.yaml turns a pre-release into a version that dpkg and rpm sort before the
# release: builds a .deb and an .rpm of a placeholder file tree (no app build) as 0.2.0-rc.1,
# 0.2.0-rc.2 and 0.2.0, reads each package's version back, and compares them with dpkg and rpm's
# own version comparison. The placeholder packages are never installed.
#
# Usage: packaging/linux/check-version-order.sh
# Needs: nfpm, dpkg-deb, rpm (packaging/linux/Dockerfile has all three).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

STAGE="$WORK/root"
mkdir -p "$STAGE/usr/bin" "$STAGE/usr/share/doc/reasoncraft"
printf 'placeholder, not a program\n' >"$STAGE/usr/bin/reasoncraft"
printf 'placeholder\n' >"$STAGE/usr/share/doc/reasoncraft/README"
sed "s|\${STAGE}|$STAGE|g" "$ROOT/packaging/linux/nfpm.yaml" >"$WORK/nfpm.yaml"
export NFPM_ARCH=amd64 REASONCRAFT_MAINTAINER="ReasonCraft contributors"

# Builds the deb and rpm of version $1; prints "<deb version> <rpm version-release>".
package_versions() {
  local version="$1" deb rpm
  deb="$WORK/v-$version.deb"
  rpm="$WORK/v-$version.rpm"
  (cd "$ROOT" && VERSION="$version" nfpm package -f "$WORK/nfpm.yaml" -p deb -t "$deb" >/dev/null)
  (cd "$ROOT" && VERSION="$version" nfpm package -f "$WORK/nfpm.yaml" -p rpm -t "$rpm" >/dev/null)
  printf '%s %s\n' "$(dpkg-deb -f "$deb" Version)" "$(rpm -qp --qf '%{VERSION}-%{RELEASE}' "$rpm")"
}

read -r deb_rc1 rpm_rc1 < <(package_versions 0.2.0-rc.1)
read -r deb_rc2 rpm_rc2 < <(package_versions 0.2.0-rc.2)
read -r deb_rel rpm_rel < <(package_versions 0.2.0)
echo "deb: $deb_rc1 < $deb_rc2 < $deb_rel"
echo "rpm: $rpm_rc1 < $rpm_rc2 < $rpm_rel"

failed=0
dpkg_lt() {
  if dpkg --compare-versions "$1" lt "$2"; then echo "PASS dpkg: $1 < $2"; else echo "FAIL dpkg: $1 is not before $2" >&2; failed=1; fi
  return 0
}
rpm_lt() {
  local cmp
  cmp="$(rpm --eval "%{lua: print(rpm.vercmp(\"$1\", \"$2\"))}")"
  if [[ "$cmp" == -1 ]]; then echo "PASS rpm: $1 < $2"; else echo "FAIL rpm: $1 is not before $2 ($cmp)" >&2; failed=1; fi
  return 0
}
dpkg_lt "$deb_rc1" "$deb_rc2"
dpkg_lt "$deb_rc2" "$deb_rel"
rpm_lt "$rpm_rc1" "$rpm_rc2"
rpm_lt "$rpm_rc2" "$rpm_rel"
exit "$failed"
