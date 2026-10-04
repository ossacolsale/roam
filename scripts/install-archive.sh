#!/bin/sh
set -eu

archive_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
exec "$archive_dir/scripts/install-artifacts.sh" \
	"$archive_dir/bin" \
	"$archive_dir/share/systemd/user/roam.service" \
	"$archive_dir/share/applications/org.roam.WifiRoaming.desktop" \
	"$archive_dir/share/icons/hicolor/scalable/apps/org.roam.WifiRoaming.svg"
