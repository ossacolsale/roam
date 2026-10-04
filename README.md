# Roam: automatic Wi-Fi roaming for Linux

Roam is a local-only, privacy-first Linux application for automatic roaming
between saved Wi-Fi networks. It watches NetworkManager and can switch between
different SSIDs, or between access points using the same SSID, when the
currently connected network becomes weak or unstable; you choose the saved
profiles it may use, and no mesh network is required.

If your Linux laptop stays connected to a weak saved Wi-Fi network while a
stronger known network is nearby, Roam is designed for that situation.

## The problem

Linux laptops can remain connected to a Wi-Fi network after its signal becomes
weak or unstable, even when another known network is available with a much
better signal. This can happen in large homes, offices, warehouses, schools,
laboratories, hotels, and other buildings with several access points or saved
Wi-Fi networks.

Roam lets you choose which saved NetworkManager Wi-Fi profiles are eligible.
It can switch to a clearly better eligible network when the current connection
deteriorates. It works with ordinary NetworkManager profiles and independent
access points: no mesh networking is required. Different SSIDs and multiple
BSSIDs under one SSID are supported.

## Who is Roam for?

- Linux laptop users moving between rooms or floors.
- People working in buildings with several Wi-Fi networks or access points.
- Users with multiple saved Wi-Fi profiles who want to choose which may be used.
- Users whose laptop stays attached to a weak network for too long.
- Anyone who wants automatic Wi-Fi switching without a mesh network.
- Users looking for a local-only solution without cloud services.

## What Roam is not

Roam is not a mesh networking system, Wi-Fi access point, VPN, cloud monitoring
service, password manager, or replacement for NetworkManager. NetworkManager
continues to manage the actual Wi-Fi connection; Roam observes it and applies
your selected roaming policy through NetworkManager.

## Install

**Available now: build from source.** No GitHub Releases or downloadable
release artifacts have been published yet. Debian/Ubuntu packages, a
Fedora-family RPM, and an Arch/AUR package are also not published. The
repository contains draft Debian, RPM, and Arch packaging definitions; they
have not been released through distro repositories or the AUR.

The GitHub release workflow is configured to create a pre-release with a
Linux x86_64 archive and checksums when a matching `v*` version tag is pushed.
That workflow has not published a release yet. The archive is not a distro
package; after extracting one, run `scripts/install-archive.sh` from the
extracted directory to install or update it.

Roam requires Linux, NetworkManager, systemd user services, GTK4, and
libadwaita. It does not support iwd-only, ConnMan, or raw `wpa_supplicant`
installations.

Install build dependencies:

```sh
# Debian/Ubuntu
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev

# Fedora
sudo dnf install gcc pkg-config gtk4-devel libadwaita-devel

# Arch Linux
sudo pacman -S base-devel pkgconf gtk4 libadwaita
```

Build and install the applications and user service (installing binaries in
`/usr/bin` requires administrator privileges; Roam itself still runs as your
user):

```sh
scripts/install-local.sh
```

Package upgrades, `scripts/install-local.sh`, and the release archive installer
stop Roam for logged-in users when it is active before replacing the binaries,
then start it again afterward. If the service was already inactive, they leave
it inactive. Package hooks cover Debian packages, RPM packages, and the Arch
PKGBUILD. They operate only on active user service managers; users who are
logged out do not have a running Roam process to restart.

The first service start creates `~/.config/roam/config.toml` with Automatic
mode, Medium responsiveness, and no eligible profiles. Open Roam and select the
saved profiles you want it to consider. The enabled user service starts at each
login and keeps running when the GUI closes. The service runs as your user and
does not require the whole application to run as root. The draft package
definitions enable the service for user logins; a first source install enables
and starts it. Launching the GUI also starts it in the current
session if needed. Logging out stops the normal user session, and the service
starts again at the next login. To opt out for this user, run
`systemctl --user mask --now roam.service`; to resume, run
`systemctl --user unmask roam.service && systemctl --user start roam.service`.

## Use

- **Mode:** Automatic switches when the policy finds a stable, clearly better
  eligible access point. Confirm asks before activation.
- **Responsiveness:** Low favors stability, Medium balances stability and
  earlier roaming, and High considers alternatives earlier.
- **Eligible networks:** Only checked saved NetworkManager profiles are
  candidates. Unknown access points are ignored.

Automatic roaming waits until the current connection has stayed degraded for
30 seconds (Low), 20 seconds (Medium), or 12 seconds (High). The alternative
must also remain visible and stable for at least 20, 12, or 8 seconds
respectively, and reach a good signal level. The candidate field shows the
best eligible alternative once it meets those signal and stability checks,
including while Roam is still waiting for the current connection's dwell time.

Roam activates a specific visible access point with NetworkManager's D-Bus
`ActivateConnection` method. It does not disconnect first, edit profiles, or
read Wi-Fi secrets. A failed activation leaves the current connection alone.

## FAQ: NetworkManager Wi-Fi roaming

### How can I automatically switch between saved Wi-Fi networks on Linux?

Roam monitors the current Wi-Fi connection and can switch between saved
NetworkManager Wi-Fi profiles selected by you when the connection becomes weak
or unstable.

### How can I automatically connect to a stronger saved Wi-Fi network on Linux?

Choose which saved Wi-Fi profiles Roam may use. It can activate a significantly
better eligible profile when the current connection deteriorates.

### How do I switch between different Wi-Fi SSIDs automatically on Linux?

Roam supports automatic switching between different saved NetworkManager Wi-Fi
profiles selected by you. It also supports multiple access points under the
same SSID.

### Does Roam require Wi-Fi mesh?

No. Roam works with ordinary independent access points and saved Wi-Fi
networks.

### Does Roam replace NetworkManager?

No. NetworkManager remains responsible for the actual Wi-Fi connection. Roam
adds a user-controlled roaming policy and asks NetworkManager to activate a
selected profile.

### Does Roam require Internet access?

No. Roam is designed to work locally and makes no cloud or API calls.

### Does Roam store Wi-Fi passwords?

No. Roam uses existing NetworkManager profiles and does not persist Wi-Fi
credentials.

## Configuration and privacy

Roam stores its mode, responsiveness, and eligible NetworkManager profile UUIDs
in `~/.config/roam/config.toml`. Signal and connection observations used by
the decision engine exist only in memory. Roam does not persist credentials,
SSID/BSSID or signal history, location, IP/DNS history, or connection history.
There is no telemetry, analytics, crash upload, cloud/API call, or update
tracking. Logs omit SSIDs, BSSIDs, passwords, and network addresses.

NetworkManager's standard D-Bus access point object reports signal strength as
a percentage quality value. Roam keeps it on that scale; it is not dBm. Policy
thresholds are internal starting points and have not been validated across real
hardware.

## Development

The workspace contains `roam-core` (policy and deterministic tests),
`roam-networkmanager` (direct D-Bus access), `roamd` (background user service
and local Unix socket IPC), and `roam-ui` (GTK4/libadwaita client). The daemon
reacts to NetworkManager D-Bus signals and uses a 30-second fallback check.
It requests scans only while seeking candidates, with a rate limit.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
cargo build --workspace
```

## Security and help

Roam uses the logged-in user's system D-Bus access to NetworkManager and does
not install a privileged helper. NetworkManager and Polkit decide whether a
requested connection activation is allowed. See [SECURITY.md](SECURITY.md) for
vulnerability reporting and [docs/index.html](docs/index.html) for the English
project landing page.

## Project links

- [Source repository](https://github.com/ossacolsale/roam)
- [GitHub Releases (none published yet)](https://github.com/ossacolsale/roam/releases)
- [Installation and development instructions](README.md#install)
- [Issue tracker](https://github.com/ossacolsale/roam/issues)
- [Privacy and security](SECURITY.md)
