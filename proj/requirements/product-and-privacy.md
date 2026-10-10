# Product and privacy requirements

> Section numbers retain their order in the original requirements analysis.

## Roam — Linux Wi-Fi roaming assistant

## 1. Goal

Build a small, privacy-first Linux desktop application that automatically moves the current Wi-Fi connection from a weak/unstable access point or SSID to a significantly better **known/saved Wi-Fi network**, when the user has selected that network as eligible for roaming.

The application must work with:

- multiple independent APs using the same SSID;
- different SSIDs that are already configured/saved in NetworkManager.

No mesh networking is required.

The key use case is:

```text
Current:
Office-WiFi        weak / unstable

Known alternatives:
Lab-WiFi            strong
Warehouse-WiFi      medium
Guest               ignored

=> switch automatically to Lab-WiFi
```

The application must make the roaming decision itself rather than relying on NetworkManager's generic connection-selection policy.

---

## 2. Product principles

The application must be:

- very small;
- simple to operate;
- local-only;
- privacy-first;
- deterministic;
- conservative against connection flapping;
- usable without root privileges whenever NetworkManager permits the required operation;
- free of cloud services, accounts and telemetry.

Do not add features that are not required for the core roaming problem.

---

## 3. Technology

Use:

- Rust for the entire application;
- GTK4 for the GUI;
- libadwaita for the UI;
- NetworkManager D-Bus API for Wi-Fi discovery and connection control;
- systemd user service for background execution.

Do NOT use:

- Flutter;
- Electron;
- a web UI;
- shelling out to `nmcli` as the primary implementation;
- polling shell commands;
- root-only networking;
- NetworkManager CLI parsing.

Use direct D-Bus communication with NetworkManager.

The implementation must be based on currently supported GTK4/libadwaita and Rust crates available in the target distributions. Keep GTK and libadwaita crate versions synchronized. Do not hard-code obsolete versions.

---

## 4. Supported platform

MVP target:

- Linux;
- NetworkManager;
- systemd user services;
- GTK4/libadwaita.

Assume a normal desktop installation.

Initial target distributions:

- Debian/Ubuntu family;
- Fedora family;
- Arch family.

Do not implement distro-specific networking code.

Do not support:

- iwd-only setups;
- raw wpa_supplicant;
- ConnMan;
- NetworkManager-less systems.

These may be future adapters but are explicitly outside the MVP.

---

## 5. Architecture

Use a small workspace:

```text
roam/
├── crates/
│   ├── roam-core/
│   ├── roam-networkmanager/
│   └── roam-ui/
├── systemd/
│   └── roam.service
├── packaging/
├── tests/
├── Cargo.toml
└── README.md
```

Responsibilities:

```text
roam-core
    decision engine
    RSSI filtering
    candidate selection
    hysteresis
    instability detection
    cooldown
    configuration model
    platform-independent tests

roam-networkmanager
    NetworkManager D-Bus integration
    saved connection profiles
    visible APs
    current connection
    Wi-Fi scan requests
    connection activation
    signal/state monitoring

roam-ui
    GTK4/libadwaita interface
    configuration
    current status
    confirmation dialogs

systemd
    keeps roaming engine running in background
```

The core must not depend on GTK or NetworkManager.

---

## 6. Background architecture

The roaming engine must continue running when the GUI is closed.

Use:

```text
systemd --user
        │
        ▼
      roamd
        │
        ├── roam-core
        │
        └── NetworkManager D-Bus
```

The GUI is a separate client:

```text
roam-ui
   │
   └── local IPC / D-Bus
             │
             ▼
           roamd
```

Keep the IPC protocol minimal.

The GUI must never be required for automatic roaming to work.

Closing the window must NOT stop the roaming service.

Provide a simple systemd user service:

```text
roam.service
```

It must not require root.

---

## 7. User-visible settings

The application has only two actual behavioural settings.

### Mode

Values:

```text
Automatic
Confirm
```

Default:

```text
Automatic
```

#### Automatic

Roam changes the connection when its decision engine determines that another selected network/AP is clearly better.

#### Confirm

Roam asks the user before activating the candidate.

The confirmation dialog must be concise:

```text
Wi-Fi signal is degrading.

Current
Office-WiFi
Weak

Better candidate
Lab-WiFi
Strong

[ Ignore ] [ Switch ]
```

Do not expose RSSI thresholds, hysteresis, timers, BSSID or other technical parameters in the normal UI.

---

### Responsiveness

User-facing values:

```text
Low
Medium
High
```

Default:

```text
Medium
```

Use the word "Responsiveness", not "Sensitivity".

Meaning:

#### Low

Prefer connection stability. Switch only when the current connection is clearly degraded or the alternative is substantially better.

#### Medium

Balanced behaviour.

#### High

Prefer earlier roaming and tolerate a smaller advantage for the candidate.

Internally this setting may affect:

- minimum candidate advantage;
- minimum current signal quality;
- candidate confirmation time;
- instability tolerance;
- post-roam cooldown.

Do not expose numeric values.

---

## 8. Third user-facing section: Eligible networks

This is not a third behavioural setting.

Provide a simple list:

```text
Networks eligible for roaming

☑ Office-WiFi
☑ Lab-WiFi
☑ Warehouse-WiFi
☐ Guest
```

This list is essential.

Only selected NetworkManager Wi-Fi connection profiles may be used as candidates for automatic roaming.

The user is expected to know which saved networks are legitimate alternatives.

Never automatically roam to an unknown/unselected network.

---

## 9. Persisted configuration

Persist only:

```text
mode
responsiveness
eligible connection profile IDs
```

Do NOT persist:

- passwords;
- Wi-Fi secrets;
- BSSID history;
- RSSI history;
- location;
- connection history;
- roaming history;
- MAC addresses unless required transiently;
- IP addresses;
- DNS information.

Do not create a database.

Use a small local configuration file in the normal XDG configuration directory.

For example:

```text
~/.config/roam/config.toml
```

The configuration must contain profile identifiers or stable NetworkManager profile references, not credentials.

Never persist secrets obtained from NetworkManager.

---

## 10. Privacy requirements

The application is strictly local.

Do not implement:

- Internet connectivity;
- cloud APIs;
- analytics;
- telemetry;
- advertising;
- user accounts;
- remote configuration;
- crash upload;
- update tracking.

The app must never send Wi-Fi information anywhere.

Logs must never contain:

- SSID;
- BSSID;
- password;
- IP address;
- geographic information.

Default logging should be minimal.

Diagnostic logs may contain anonymized technical state, for example:

```text
candidate_count=3
current_signal=-74
candidate_delta=12
decision=ROAM
```

but should not include identifiable Wi-Fi names or BSSIDs.

---

