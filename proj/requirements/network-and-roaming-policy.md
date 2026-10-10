# Network and roaming policy requirements

> Section numbers retain their order in the original requirements analysis.

## 11. NetworkManager integration

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

## 12. Saved networks

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

## 13. Access points

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

## 14. Current connection

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

## 15. Candidate definition

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

## 16. Same SSID roaming

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

## 17. Different SSID roaming

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

## 18. Activation

When the engine decides to roam, activate the selected NetworkManager connection profile and, when appropriate, specify the target AP/BSSID.

NetworkManager supports activation against a specific Wi-Fi AP object. Use this mechanism rather than merely asking NetworkManager to choose any AP for the profile.

Reference:

NetworkManager D-Bus `ActivateConnection(connection, device, specific_object)` supports a Wi-Fi AP object as the `specific_object`.

---

## 19. Never disconnect first

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

## 20. Decision engine

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

## 21. Signal smoothing

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

## 22. Instability detection

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

## 23. Candidate superiority

A candidate should normally satisfy:

```text
candidate_quality > current_quality + hysteresis
```

Use a meaningful margin.

Initial internal starting points:

#### Low

```text
candidate advantage >= 10 dB
```

#### Medium

```text
candidate advantage >= 8 dB
```

#### High

```text
candidate advantage >= 6 dB
```

These are starting values only. Keep them centralized in the policy configuration so they can be tuned after real-world testing.

Do not expose them to the user.

---

## 24. Absolute signal thresholds

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

## 25. Candidate stability

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

## 26. Cooldown

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

## 27. Avoiding ping-pong

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

## 28. Network selection policy

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

## 29. Scan strategy

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

## 30. Event-driven design

Prefer NetworkManager D-Bus signals/properties over tight polling.

Use polling only for periodic policy evaluation.

The engine should react to:

- connection changes;
- AP changes;
- signal changes;
- profile changes;
- device state changes.

---

## 31. Confirmation mode

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

## 32. Automatic mode

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

## 33. Failure handling

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

## 34. Security

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

