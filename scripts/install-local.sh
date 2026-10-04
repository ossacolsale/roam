#!/bin/sh
set -eu

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_dir"
cargo build --release --workspace --locked
exec "$repo_dir/scripts/install-artifacts.sh" \
	"$repo_dir/target/release" \
	"$repo_dir/systemd/roam.service" \
	"$repo_dir/packaging/org.roam.WifiRoaming.desktop" \
	"$repo_dir/assets/icons/hicolor/scalable/apps/org.roam.WifiRoaming.svg"
