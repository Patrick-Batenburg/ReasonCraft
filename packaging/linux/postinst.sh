#!/bin/sh
# Adapted from WordCraft (https://github.com/storytold/wordcraft, packaging/linux/postinst.sh at
# aadb077), used under MIT OR Apache-2.0; changed: this header only.
#
# deb/rpm post-install and post-remove: refresh the desktop, MIME and icon caches.
# Every tool is optional; missing ones are skipped.
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications || true
fi
if command -v update-mime-database >/dev/null 2>&1; then
  update-mime-database /usr/share/mime || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi
exit 0
