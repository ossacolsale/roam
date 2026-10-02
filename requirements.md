# ROAM — Linux Wi-Fi Roaming Assistant

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

# 2. Product principles

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

# 3. Technology

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

# 4. Supported platform

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

# 5. Architecture

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

# 6. Background architecture

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

# 7. User-visible settings

The application has only two actual behavioural settings.

## Mode

Values:

```text
Automatic
Confirm
```

Default:

```text
Automatic
```

### Automatic

Roam changes the connection when its decision engine determines that another selected network/AP is clearly better.

### Confirm

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

## Responsiveness

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

### Low

Prefer connection stability. Switch only when the current connection is clearly degraded or the alternative is substantially better.

### Medium

Balanced behaviour.

### High

Prefer earlier roaming and tolerate a smaller advantage for the candidate.

Internally this setting may affect:

- minimum candidate advantage;
- minimum current signal quality;
- candidate confirmation time;
- instability tolerance;
- post-roam cooldown.

Do not expose numeric values.

---

# 8. Third user-facing section: Eligible networks

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

# 9. Persisted configuration

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

# 10. Privacy requirements

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

# 11. NetworkManager integration

Use NetworkManager D-Bus directly.

The backend must discover:

1. Wi-Fi devices;
2. currently active connection;
3. saved Wi-Fi connection profiles;
4. currently visible access points;
5. AP signal strength;
6. SSID;
7. BSSID;
8. frequency/band where available;
9. security information where useful;
10. connection state.

Use NetworkManager's native object model.

Do not repeatedly execute:

```text
nmcli ...
```

from the application.

---

# 12. Saved networks

Enumerate Wi-Fi connection profiles from NetworkManager.

For each profile expose internally:

```rust
SavedNetwork {
    profile_path,
    uuid,
    id,
    ssid,
    security,
}
```

Never retrieve secrets/passwords unless absolutely required by NetworkManager itself.

Roam must not need to know Wi-Fi passwords.

A saved profile becomes a candidate only when the user explicitly enables it in the "Eligible networks" list.

---

# 13. Access points

Represent visible APs as:

```rust
AccessPoint {
    object_path,
    ssid,
    bssid,
    frequency_mhz,
    signal_dbm,
    security,
    last_seen,
}
```

The exact mapping of NetworkManager's signal representation to dBm must be handled carefully. If only a percentage is exposed by a given API path, do not incorrectly label it as dBm.

Prefer the strongest available native signal representation.

---

# 14. Current connection

Represent the current Wi-Fi connection as:

```rust
CurrentConnection {
    profile_path,
    profile_uuid,
    ssid,
    bssid,
    frequency_mhz,
    signal,
}
```

The current BSSID is important because an SSID may contain multiple APs.

---

# 15. Candidate definition

A candidate is valid only if:

1. the AP belongs to a saved NetworkManager profile;
2. that profile is selected by the user as an eligible roaming network;
3. the AP is currently visible/recently observed;
4. the profile can actually be activated;
5. it is not the exact current AP;
6. the candidate signal/quality is sufficiently better;
7. the candidate has remained sufficiently stable;
8. the cooldown rules permit a switch.

Unknown Wi-Fi networks are never candidates.

---

# 16. Same SSID roaming

Example:

```text
Office-WiFi
    AP A   -76
    AP B   -54
    AP C   -81
```

The application must treat another BSSID of the same profile as a valid roaming candidate.

The user does not need to configure individual BSSIDs.

---

# 17. Different SSID roaming

Example:

```text
Current:
Office-WiFi     -76

Eligible:
Lab-WiFi        -52
Warehouse-WiFi  -68
```

If Lab-WiFi is selected by the user, Roam may activate its saved NetworkManager profile.

This is a primary product requirement.

---

# 18. Activation

When the engine decides to roam, activate the selected NetworkManager connection profile and, when appropriate, specify the target AP/BSSID.

NetworkManager supports activation against a specific Wi-Fi AP object. Use this mechanism rather than merely asking NetworkManager to choose any AP for the profile.

Reference:

NetworkManager D-Bus `ActivateConnection(connection, device, specific_object)` supports a Wi-Fi AP object as the `specific_object`.

---

# 19. Never disconnect first

Do not implement:

```text
disconnect current
wait
connect candidate
```

unless NetworkManager absolutely requires it.

Prefer a direct activation request.

The goal is to minimize interruption.

If activation fails, do not intentionally destroy the existing working connection.

---

# 20. Decision engine

The core algorithm must NOT be:

```text
if candidate.signal > current.signal:
    switch
```

This would cause excessive roaming/flapping.

Use a state machine.

Suggested states:

```text
STABLE
DEGRADING
SEEKING
CANDIDATE_FOUND
AWAITING_CONFIRMATION
ROAMING
COOLDOWN
```

---

# 21. Signal smoothing

Use a lightweight exponential moving average (EMA).

Example:

```text
filtered = alpha * current + (1 - alpha) * previous
```

Start with a conservative alpha around:

```text
0.3
```

Make the value internal and easy to tune.

Do not expose it in the UI.

---

# 22. Instability detection

Do not look only at absolute signal.

Detect:

- sustained degradation;
- repeated rapid drops;
- high short-term variation;
- a signal that repeatedly crosses the usable/poor boundary.

Example:

```text
-58
-61
-67
-73
-69
-76
```

may indicate instability/degradation even if the instantaneous value occasionally improves.

Use a short rolling history in RAM.

No history is written to disk.

---

# 23. Candidate superiority

A candidate should normally satisfy:

```text
candidate_quality > current_quality + hysteresis
```

Use a meaningful margin.

Initial internal starting points:

### Low

```text
candidate advantage >= 10 dB
```

### Medium

```text
candidate advantage >= 8 dB
```

### High

```text
candidate advantage >= 6 dB
```

These are starting values only. Keep them centralized in the policy configuration so they can be tuned after real-world testing.

Do not expose them to the user.

---

# 24. Absolute signal thresholds

Use absolute signal thresholds only as part of the decision, not as the sole trigger.

Initial starting point:

```text
strong/good:      above approximately -65 dBm
degraded:         approximately -65 to -72 dBm
poor:             below approximately -72 dBm
very poor:        below approximately -78 dBm
```

Do not assume these values are universally optimal.

They are initial engineering defaults.

The candidate advantage and trend/stability should matter more than a single absolute RSSI measurement.

Keep thresholds centralized for later tuning.

---

# 25. Candidate stability

Do not switch because a candidate appears strong in one scan.

Require the candidate to remain valid for multiple observations.

Initial defaults:

```text
Low:      3 consecutive confirmations
Medium:   2
High:     2
```

Where possible, use timestamp-based confirmation rather than assuming a fixed scan interval.

---

# 26. Cooldown

After successful roaming, enter cooldown.

Initial defaults:

```text
Low:      90 s
Medium:   60 s
High:     30 s
```

During cooldown, do not perform another normal roam.

Exception:

If the current connection becomes critically unusable and another valid candidate is clearly superior, allow an emergency roam.

---

# 27. Avoiding ping-pong

The engine must prevent:

```text
A -> B -> A -> B
```

caused by small signal fluctuations.

Use all of:

- hysteresis;
- filtered signal;
- candidate stability;
- post-roam cooldown.

A candidate that was just abandoned should receive a temporary internal penalty unless the current connection becomes clearly worse.

---

# 28. Network selection policy

When multiple eligible candidates exist, rank them by:

1. valid eligible NetworkManager profile;
2. candidate signal/quality;
3. stability;
4. advantage over current connection;
5. recent failure penalty;
6. band/frequency only as secondary information.

Do not automatically prefer 5 GHz over 2.4 GHz.

Do not automatically prefer a stronger RSSI if another candidate has demonstrated instability.

Do not hard-code a vendor-specific Wi-Fi policy.

---

# 29. Scan strategy

Do not continuously force Wi-Fi scans at a high frequency.

Prefer:

1. NetworkManager's currently available AP information;
2. NetworkManager-generated changes/events;
3. explicit scans only when necessary.

Suggested behaviour:

```text
stable:
    low scan pressure

degrading:
    more frequent evaluation

candidate search:
    refresh AP information if stale

roaming/cooldown:
    reduce scan pressure
```

Respect NetworkManager and driver scan limitations.

The engine must continue functioning if scans fail temporarily.

A failed scan must never trigger a disconnect.

---

# 30. Event-driven design

Prefer NetworkManager D-Bus signals/properties over tight polling.

Use polling only for periodic policy evaluation.

The engine should react to:

- connection changes;
- AP changes;
- signal changes;
- profile changes;
- device state changes.

---

# 31. Confirmation mode

When mode is `Confirm`:

```text
candidate found
      ↓
show notification/dialog
      ↓
user:
  Switch
      OR
  Ignore
```

Do not switch before the user confirms.

If the user ignores the candidate:

- suppress that exact candidate for a short internal period;
- continue monitoring;
- do not repeatedly prompt every scan.

Suggested suppression:

```text
2–5 minutes
```

Keep it internal.

---

# 32. Automatic mode

When mode is `Automatic`:

```text
valid candidate
        +
stable candidate
        +
sufficient advantage
        +
current connection degraded enough
        +
not in cooldown
        ↓
activate candidate
```

After activation:

- verify that the new connection actually became active;
- verify that the resulting connection is healthy;
- only then enter normal cooldown.

If activation fails:

- preserve the old state if still connected;
- apply a temporary failure penalty to that candidate;
- do not repeatedly retry immediately.

---

# 33. Failure handling

Never intentionally leave the user without connectivity merely because roaming failed.

If candidate activation fails:

```text
current connection still healthy
    => stay connected

current connection unhealthy
    => continue candidate evaluation
```

Avoid retry loops.

---

# 34. Security

Never modify or delete existing NetworkManager profiles.

Never change passwords.

Never modify DNS.

Never modify routes.

Never modify proxy settings.

Never alter unrelated connection properties.

Roam's responsibility is only:

```text
observe Wi-Fi
select eligible candidate
activate candidate
```

---

# 35. UI

The main window should fit roughly one small desktop window.

Suggested layout:

```text
ROAM

● Monitoring active

MODE
[ Automatic ] [ Confirm ]

RESPONSIVENESS
[ Low ] [ Medium ] [ High ]

ELIGIBLE NETWORKS
☑ Office-WiFi
☑ Lab-WiFi
☐ Guest
☑ Warehouse-WiFi

────────────────────

Current
Office-WiFi
Weak

Candidate
Lab-WiFi
Strong

```

Do not create tabs for the MVP.

Do not create charts.

Do not expose technical settings.

---

# 36. Status language

Use human-readable descriptions:

```text
Strong
Good
Weak
Very weak
Searching
Switching
```

Do not make the user understand dBm.

dBm may be available only in a future diagnostics screen.

---

# 37. Tray / background UI

Do not make desktop tray integration a dependency of the MVP.

The daemon must run independently of the GUI.

The GUI can simply show whether the daemon is active.

A tray icon can be added later if a clean cross-desktop implementation is found.

---

# 38. Accessibility

Use standard GTK4/libadwaita widgets.

Keyboard navigation must work.

All controls need accessible labels.

Do not rely solely on color to convey signal state.

---

# 39. Persistence and configuration

Use a small TOML configuration file.

Example:

```toml
mode = "automatic"
responsiveness = "medium"

eligible_profiles = [
    "uuid-1",
    "uuid-2"
]
```

Do not store SSIDs when a stable NetworkManager profile UUID/path is sufficient.

When displaying the configuration, resolve the UUID to the current SSID/profile name.

If a configured profile disappears:

- keep the UUID in config;
- show it as unavailable;
- do not crash;
- do not silently substitute another network.

---

# 40. Logging

Default:

```text
INFO
```

No Wi-Fi identifiers in logs.

Example acceptable log:

```text
roaming decision: candidate_count=2 result=stay
```

Example forbidden log:

```text
switching from Office-WiFi to Lab-WiFi
BSSID aa:bb:cc:dd:ee:ff
```

unless an explicit future diagnostic mode is enabled.

---

# 41. Tests

The decision engine must be extensively unit tested without NetworkManager.

Create deterministic simulated scenarios.

## Scenario A — healthy connection

```text
current:
-55
-57
-58
-59
```

Expected:

```text
STAY
```

## Scenario B — current weak, candidate clearly better

```text
current:
-60
-67
-72
-76

candidate:
-53
-52
-54
```

Expected:

```text
ROAM
```

## Scenario C — small advantage

```text
current = -67
candidate = -69
```

Expected:

```text
STAY
```

## Scenario D — candidate fluctuates

```text
candidate:
-50
-73
-51
-72
```

Expected:

```text
STAY
```

## Scenario E — ping-pong

Verify:

```text
A -> B
```

does not immediately produce:

```text
B -> A
```

unless B has genuinely degraded.

## Scenario F — unknown network

```text
unknown AP = -35
eligible AP = -65
```

Expected:

```text
ignore unknown AP
```

## Scenario G — different eligible SSID

```text
current:
Office = -75

candidate:
Lab = -52
```

Expected:

```text
ROAM
```

assuming Lab is enabled in the user's eligible network list.

## Scenario H — confirmation mode

Expected:

```text
candidate detected
=> no activation
=> confirmation required
```

## Scenario I — failed activation

Expected:

```text
activation failure
=> no crash
=> no endless retries
=> preserve existing connection if possible
```

## Scenario J — cooldown

Verify that a successful roam prevents immediate oscillation.

---

# 42. Integration tests

Where practical, provide an integration-test layer for NetworkManager.

Do not require an actual Wi-Fi card for normal CI.

Separate:

```text
core tests
```

from:

```text
NetworkManager integration tests
```

NetworkManager integration tests may require a specific Linux environment and can be marked accordingly.

---

# 43. Fake backend

Implement a `FakeWifiBackend` for deterministic tests.

It must be able to simulate:

- current network;
- AP list;
- RSSI changes;
- scan results;
- successful activation;
- activation failure;
- connection loss;
- multiple profiles;
- multiple BSSIDs for one SSID.

This fake backend is important.

Do not write the algorithm directly against D-Bus.

---

# 44. NetworkManager backend interface

Define an abstraction similar to:

```rust
trait WifiBackend {
    fn current_connection(&self) -> Result<Option<CurrentConnection>>;
    fn saved_networks(&self) -> Result<Vec<SavedNetwork>>;
    fn access_points(&self) -> Result<Vec<AccessPoint>>;
    fn request_scan(&self) -> Result<()>;
    fn activate(
        &self,
        profile: &SavedNetwork,
        device: &WifiDevice,
        ap: Option<&AccessPoint>,
    ) -> Result<()>;
}
```

Exact implementation details may differ according to the current NetworkManager D-Bus API.

---

# 45. Permissions and Polkit

Do not assume root access.

Attempt normal NetworkManager operations as the logged-in user.

If NetworkManager/Polkit denies an operation:

- expose a clear error;
- do not suggest running the whole application as root;
- do not silently fall back to unsafe shell commands.

Document which NetworkManager/Polkit permissions are required.

---

# 46. Packaging

MVP should provide:

1. source build;
2. systemd user service installation;
3. `.desktop` file;
4. basic Debian packaging;
5. basic RPM packaging if practical.

Do not make Flatpak the primary packaging target for the first implementation because the application needs deep integration with the host NetworkManager service and a background user service.

Packaging must not require root for runtime operation.

Installation may require administrative privileges; runtime should not.

---

# 47. Autostart

Provide an option during installation or first launch:

```text
Start automatically with my session
```

Default:

```text
enabled
```

This should enable the systemd user service.

The roaming engine must start automatically without opening the main window.

---

# 48. First launch

First launch should be minimal:

```text
ROAM

Roam automatically between the Wi-Fi networks
you choose when the current connection becomes weak.

[ Continue ]
```

Then:

```text
Choose networks eligible for roaming

☑ Office-WiFi
☑ Lab-WiFi
☐ Guest

[ Done ]
```

Then normal main screen.

Do not ask for:

- passwords;
- account information;
- location;
- permissions unrelated to NetworkManager.

---

# 49. No hidden behaviour

The application must never:

- connect to an unselected Wi-Fi;
- change network settings unrelated to roaming;
- modify profiles;
- delete profiles;
- access Wi-Fi secrets unnecessarily;
- collect telemetry;
- contact external servers.

---

# 50. MVP scope

Implement only:

### Core

- state machine;
- RSSI filtering;
- degradation detection;
- instability detection;
- candidate ranking;
- hysteresis;
- cooldown;
- Low/Medium/High policies;
- Automatic/Confirm modes.

### NetworkManager

- current connection;
- saved profiles;
- AP scanning/observation;
- AP signal;
- activation of a specific saved profile/AP;
- connection state monitoring.

### GUI

- mode;
- responsiveness;
- eligible networks;
- current status;
- confirmation dialog;
- monitoring state.

### Runtime

- systemd user service;
- local IPC;
- persistent configuration;
- minimal logging.

---

# 51. Explicitly OUT OF SCOPE

Do NOT implement in MVP:

- Android;
- Windows;
- iwd;
- wpa_supplicant direct integration;
- mesh management;
- VPN;
- network diagnostics suite;
- speed tests;
- charts;
- historical statistics;
- cloud;
- telemetry;
- accounts;
- remote control;
- automatic firmware/router configuration;
- password management;
- tray icon dependency;
- advanced expert settings.

---

# 52. Development order

Implement in this exact order:

## Phase 1

Create Rust workspace and pure `roam-core`.

Implement all policy logic using the fake backend.

Write comprehensive unit tests.

No GUI and no D-Bus yet.

## Phase 2

Implement NetworkManager D-Bus backend.

Verify:

- current connection;
- saved Wi-Fi profiles;
- AP discovery;
- AP signal;
- target activation.

## Phase 3

Connect real NetworkManager backend to the core.

Test on a real multi-AP environment.

## Phase 4

Implement GTK4/libadwaita GUI.

## Phase 5

Implement systemd user service and local IPC.

## Phase 6

Package for Debian/Ubuntu.

Then add Fedora/Arch packaging.

---

# 53. Important engineering rule

Do not prematurely optimize the algorithm.

First make this sequence reliable:

```text
detect degradation
        ↓
find better eligible AP/network
        ↓
verify candidate
        ↓
activate candidate
        ↓
verify successful connection
        ↓
cooldown
```

Then tune the thresholds using real-world measurements.

The product's value is not a sophisticated UI.

The product's value is making the roaming decision more reliably than the default behaviour in environments where multiple known Wi-Fi networks/APs coexist.

---

# 54. Definition of done for MVP

The MVP is successful when, on a real Linux laptop:

1. the user selects several saved Wi-Fi networks;
2. the user chooses Automatic + Medium;
3. Roam runs in the background;
4. the user walks from the coverage area of network/AP A toward B;
5. A degrades significantly;
6. B is significantly better;
7. Roam identifies B as an eligible candidate;
8. Roam switches to B automatically;
9. the switch does not create repeated A/B oscillation;
10. the user's Wi-Fi passwords never enter Roam's persistent storage;
11. no network information leaves the machine;
12. the GUI remains simple enough to understand without technical Wi-Fi knowledge.

For Confirm mode:

1. all of the above applies;
2. Roam must stop before activation;
3. the user explicitly chooses `Switch`.

---

# 55. Deliverables

Codex must produce:

- complete Rust source;
- Cargo workspace;
- unit tests;
- fake backend;
- NetworkManager D-Bus backend;
- GTK4/libadwaita GUI;
- systemd user service;
- desktop entry;
- example configuration;
- installation instructions;
- development instructions;
- privacy statement;
- troubleshooting section;
- notes explaining NetworkManager/Polkit requirements.

Do not generate placeholder implementations for the core roaming logic.

The first executable milestone must be usable with the fake backend and must have passing tests before the GUI/backend integration is considered complete.

# End