# Roam

Roam is a small, local Linux Wi-Fi roaming assistant. The background `roamd`
service watches NetworkManager and can activate only saved Wi-Fi profiles that
you explicitly mark as eligible. The GTK4/libadwaita window is a separate
client; closing it does not stop roaming.

## Install

Roam requires a normal desktop session with NetworkManager, systemd user
services, GTK4, and libadwaita. It does not support iwd-only, ConnMan, or raw
`wpa_supplicant` installations.

Build from source with a current stable Rust toolchain and the GTK development
packages for your distribution. On Debian/Ubuntu install `libgtk-4-dev`,
`libadwaita-1-dev`, `build-essential`, and `pkg-config`; Fedora uses
`gtk4-devel`, `libadwaita-devel`, and `gcc`; Arch uses `gtk4`, `libadwaita`, and
`base-devel`.

```sh
cargo build --release --workspace
install -Dm755 target/release/roamd ~/.local/bin/roamd
install -Dm755 target/release/roam-ui ~/.local/bin/roam-ui
install -Dm644 systemd/roam.service ~/.config/systemd/user/roam.service
install -Dm644 packaging/org.roam.WifiRoaming.desktop ~/.local/share/applications/org.roam.WifiRoaming.desktop
systemctl --user daemon-reload
systemctl --user enable --now roam.service
```

The first launch creates `~/.config/roam/config.toml` with Automatic mode,
Medium responsiveness, and no eligible profiles. Open Roam and select the
saved profiles you want to use. The installation enables the user service by
default; runtime networking does not require root.

## Use

- **Mode:** Automatic switches when the policy finds a stable, clearly better
  eligible access point. Confirm asks before activation.
- **Responsiveness:** Low favors stability, Medium balances stability and
  earlier roaming, and High considers alternatives earlier.
- **Networks eligible for roaming:** Only checked saved NetworkManager profiles
  can be selected as candidates. Unknown access points are ignored.

The service activates a specific visible access point through NetworkManager's
D-Bus `ActivateConnection` call. It does not disconnect first, edit profiles,
or read Wi-Fi secrets.

## Configuration

Roam stores only its mode, responsiveness, and eligible NetworkManager profile
UUIDs in the XDG configuration directory, normally
`~/.config/roam/config.toml`. The file and local IPC socket are restricted to
the logged-in user. The included example is in
[`packaging/example-config.toml`](packaging/example-config.toml).

## Development

The workspace contains:

- `roam-core`: pure roaming policy, in-memory filtering/state, and a fake Wi-Fi
  backend for deterministic tests;
- `roam-networkmanager`: direct NetworkManager D-Bus objects and activation;
- `roamd`: the background user service, config persistence, and local Unix
  socket IPC;
- `roam-ui`: the GTK4/libadwaita client.

Run the policy tests with `cargo test -p roam-core`. Build all applications with
`cargo build --workspace`. GTK and libadwaita are runtime/build dependencies of
the UI crate; core tests do not need a Wi-Fi adapter or a running
NetworkManager daemon.

## Privacy

Roam is local-only. It has no accounts, cloud calls, telemetry, analytics,
crash uploads, or update tracking. It stores no passwords, BSSID history,
signal history, connection history, location, IP addresses, or DNS data. Signal
history exists only in memory for the decision engine. Logs omit SSIDs and
BSSIDs.

NetworkManager's standard D-Bus access point object provides signal strength
as a percentage. Roam keeps this as a percentage quality value and never
labels it dBm. The policy thresholds are internal starting points and should
be tuned with real hardware before broad distribution.

## Troubleshooting

- **Monitoring service is not running:** Run
  `systemctl --user status roam.service`, then
  `systemctl --user restart roam.service`.
- **No saved networks appear:** Confirm the Wi-Fi profiles exist in
  NetworkManager and that the device is managed by NetworkManager.
- **A switch is denied:** Roam makes normal user-session D-Bus requests. Your
  NetworkManager and Polkit policy must allow the logged-in user to activate
  saved connections. Do not run the whole application as root. If policy denies
  activation, the current connection is left alone and the service logs a
  generic failure without Wi-Fi identifiers.
- **No AP candidate appears:** Confirm the profile is checked and visible.
  NetworkManager scan results can be temporarily stale or restricted by the
  driver; failed scans never cause a disconnect.

## NetworkManager and Polkit

The application talks to the system NetworkManager D-Bus service using the
logged-in user's session credentials. Roam needs permission to enumerate
devices/profiles, request scans, read visible AP properties, and activate an
existing saved profile. Existing system Polkit policy normally governs the
activation. Roam does not install a privileged helper and does not request
passwords or secrets.

## Packaging

`debian/` contains a basic Debian source package definition and
`packaging/roam.spec` provides a basic RPM spec for Fedora-family systems.
Install-time service enabling uses the user session; the service itself runs as
the user.
