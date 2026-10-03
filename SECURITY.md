# Security policy

## Reporting a vulnerability

Please report security vulnerabilities privately to the repository owner using
GitHub's **Report a vulnerability** feature on the
[Security tab](https://github.com/ossacolsale/roam/security). Include the
affected version, impact, and steps to reproduce. Please do not open a public
issue for an unpatched vulnerability. If private vulnerability reporting is
unavailable, contact the maintainer through the GitHub profile and include no
exploit details in a public message.

## Security design

Roam does not manage or persist Wi-Fi passwords. It uses existing
NetworkManager profiles and does not modify unrelated profile properties.
The daemon and desktop application run as the logged-in user; the entire
application does not need to run as root. Roam uses normal system D-Bus and
NetworkManager mechanisms, with existing NetworkManager/Polkit policy deciding
whether activation is permitted. If a requested activation fails, Roam leaves
the current connection alone.

Roam stores selected profile UUIDs and user preferences locally. It does not
store SSID/BSSID, signal, location, IP, or DNS history and has no telemetry,
analytics, crash upload, cloud service, or external update tracking.
