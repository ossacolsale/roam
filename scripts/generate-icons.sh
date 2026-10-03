#!/bin/sh
set -eu

source_icon="assets/icons/hicolor/scalable/apps/org.roam.WifiRoaming.svg"
if command -v rsvg-convert >/dev/null 2>&1; then
  renderer=rsvg
elif command -v ffmpeg >/dev/null 2>&1; then
  renderer=ffmpeg
else
  echo "Install librsvg (rsvg-convert) or FFmpeg to generate icon PNGs." >&2
  exit 1
fi

for size in 16 24 32 48 64 128 256 512; do
  destination="assets/icons/hicolor/${size}x${size}/apps/org.roam.WifiRoaming.png"
  mkdir -p "$(dirname "$destination")"
  if [ "$renderer" = rsvg ]; then
    rsvg-convert -w "$size" -h "$size" "$source_icon" > "$destination"
  else
    ffmpeg -hide_banner -loglevel error -y -i "$source_icon" -frames:v 1 \
      -vf "scale=${size}:${size}" "$destination"
  fi
done
