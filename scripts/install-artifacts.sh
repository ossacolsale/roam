#!/bin/sh
set -eu

if [ "$#" -ne 4 ]; then
	echo "Usage: $0 BIN_DIR SERVICE_FILE DESKTOP_FILE ICON_FILE" >&2
	exit 2
fi

binary_dir=$1
service_file=$2
desktop_file=$3
icon_file=$4
for file in "$binary_dir/roamd" "$binary_dir/roam-ui" \
	"$service_file" "$desktop_file" "$icon_file"; do
	[ -f "$file" ] || { echo "Missing install file: $file" >&2; exit 1; }
done

was_active=0
had_unit=0
if systemctl --user cat roam.service >/dev/null 2>&1; then
	had_unit=1
fi
if systemctl --user is-active --quiet roam.service; then
	was_active=1
	systemctl --user stop roam.service
fi

restore_service() {
	if [ "$was_active" -eq 1 ]; then
		systemctl --user daemon-reload || true
		systemctl --user start roam.service || true
	fi
}
trap restore_service EXIT
trap 'exit 1' HUP INT TERM

sudo install -Dm755 "$binary_dir/roamd" /usr/bin/roamd
sudo install -Dm755 "$binary_dir/roam-ui" /usr/bin/roam-ui
install -Dm644 "$service_file" "$HOME/.config/systemd/user/roam.service"
install -Dm644 "$desktop_file" \
	"$HOME/.local/share/applications/org.roam.WifiRoaming.desktop"
install -Dm644 "$icon_file" \
	"$HOME/.local/share/icons/hicolor/scalable/apps/org.roam.WifiRoaming.svg"

systemctl --user daemon-reload
if [ "$was_active" -eq 1 ]; then
	systemctl --user start roam.service
	was_active=0
elif [ "$had_unit" -eq 0 ]; then
	systemctl --user enable --now roam.service
fi
trap - EXIT HUP INT TERM
