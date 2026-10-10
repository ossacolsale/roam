# Application and validation requirements

> Section numbers retain their order in the original requirements analysis.

## 35. UI

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

## 36. Status language

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

## 37. Tray / background UI

Do not make desktop tray integration a dependency of the MVP.

The daemon must run independently of the GUI.

The GUI can simply show whether the daemon is active.

A tray icon can be added later if a clean cross-desktop implementation is found.

---

## 38. Accessibility

Use standard GTK4/libadwaita widgets.

Keyboard navigation must work.

All controls need accessible labels.

Do not rely solely on color to convey signal state.

---

## 39. Persistence and configuration

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

## 40. Logging

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

## 41. Tests

The decision engine must be extensively unit tested without NetworkManager.

Create deterministic simulated scenarios.

### Scenario A — healthy connection

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

### Scenario B — current weak, candidate clearly better

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

### Scenario C — small advantage

```text
current = -67
candidate = -69
```

Expected:

```text
STAY
```

### Scenario D — candidate fluctuates

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

### Scenario E — ping-pong

Verify:

```text
A -> B
```

does not immediately produce:

```text
B -> A
```

unless B has genuinely degraded.

### Scenario F — unknown network

```text
unknown AP = -35
eligible AP = -65
```

Expected:

```text
ignore unknown AP
```

### Scenario G — different eligible SSID

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

### Scenario H — confirmation mode

Expected:

```text
candidate detected
=> no activation
=> confirmation required
```

### Scenario I — failed activation

Expected:

```text
activation failure
=> no crash
=> no endless retries
=> preserve existing connection if possible
```

### Scenario J — cooldown

Verify that a successful roam prevents immediate oscillation.

---

## 42. Integration tests

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

## 43. Fake backend

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

## 44. NetworkManager backend interface

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

## 45. Permissions and Polkit

Do not assume root access.

Attempt normal NetworkManager operations as the logged-in user.

If NetworkManager/Polkit denies an operation:

- expose a clear error;
- do not suggest running the whole application as root;
- do not silently fall back to unsafe shell commands.

Document which NetworkManager/Polkit permissions are required.

---

## 46. Packaging

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

## 47. Autostart

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

## 48. First launch

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

## 49. No hidden behaviour

The application must never:

- connect to an unselected Wi-Fi;
- change network settings unrelated to roaming;
- modify profiles;
- delete profiles;
- access Wi-Fi secrets unnecessarily;
- collect telemetry;
- contact external servers.

---

## 50. MVP scope

Implement only:

#### Core

- state machine;
- RSSI filtering;
- degradation detection;
- instability detection;
- candidate ranking;
- hysteresis;
- cooldown;
- Low/Medium/High policies;
- Automatic/Confirm modes.

#### NetworkManager

- current connection;
- saved profiles;
- AP scanning/observation;
- AP signal;
- activation of a specific saved profile/AP;
- connection state monitoring.

#### GUI

- mode;
- responsiveness;
- eligible networks;
- current status;
- confirmation dialog;
- monitoring state.

#### Runtime

- systemd user service;
- local IPC;
- persistent configuration;
- minimal logging.

---

## 51. Explicitly OUT OF SCOPE

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

## 52. Development order

Implement in this exact order:

### Phase 1

Create Rust workspace and pure `roam-core`.

Implement all policy logic using the fake backend.

Write comprehensive unit tests.

No GUI and no D-Bus yet.

### Phase 2

Implement NetworkManager D-Bus backend.

Verify:

- current connection;
- saved Wi-Fi profiles;
- AP discovery;
- AP signal;
- target activation.

### Phase 3

Connect real NetworkManager backend to the core.

Test on a real multi-AP environment.

### Phase 4

Implement GTK4/libadwaita GUI.

### Phase 5

Implement systemd user service and local IPC.

### Phase 6

Package for Debian/Ubuntu.

Then add Fedora/Arch packaging.

---

## 53. Important engineering rule

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

## 54. Definition of done for MVP

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

## 55. Deliverables

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

## End