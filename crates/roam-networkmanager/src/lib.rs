//! Direct NetworkManager D-Bus integration. No NetworkManager CLI is used.

use anyhow::{anyhow, Context, Result};
use roam_core::{AccessPoint, CurrentConnection, SavedNetwork};
use std::collections::HashMap;
use zbus::blocking::{Connection, Proxy};
use zvariant::{OwnedObjectPath, OwnedValue};

const NM: &str = "org.freedesktop.NetworkManager";
const ROOT: &str = "/org/freedesktop/NetworkManager";
const DEV_IFACE: &str = "org.freedesktop.NetworkManager.Device";
const WIFI_IFACE: &str = "org.freedesktop.NetworkManager.Device.Wireless";
const AP_IFACE: &str = "org.freedesktop.NetworkManager.AccessPoint";
const SETTINGS: &str = "/org/freedesktop/NetworkManager/Settings";
const SETTINGS_IFACE: &str = "org.freedesktop.NetworkManager.Settings";
const CONNECTION_IFACE: &str = "org.freedesktop.NetworkManager.Settings.Connection";

#[derive(Clone, Debug)]
pub struct WifiDevice {
    pub path: OwnedObjectPath,
}

#[derive(Clone, Debug)]
pub struct ApDetails {
    pub core: AccessPoint,
    pub path: OwnedObjectPath,
}

#[derive(Clone, Debug)]
pub struct ProfileDetails {
    pub core: SavedNetwork,
    pub path: OwnedObjectPath,
    pub security: String,
}

/// Backend abstraction that keeps NetworkManager operations separate from policy.
pub trait WifiBackend {
    fn wifi_devices(&self) -> Result<Vec<WifiDevice>>;
    fn saved_networks(&self) -> Result<Vec<ProfileDetails>>;
    fn access_points(
        &self,
        device: &WifiDevice,
        profiles: &[ProfileDetails],
        now: u64,
    ) -> Result<Vec<ApDetails>>;
    fn current_connection(
        &self,
        device: &WifiDevice,
        profiles: &[ProfileDetails],
    ) -> Result<Option<CurrentConnection>>;
    fn request_scan(&self, device: &WifiDevice) -> Result<()>;
    fn activate(
        &self,
        profile: &ProfileDetails,
        device: &WifiDevice,
        ap: Option<&ApDetails>,
    ) -> Result<()>;
}

pub struct NetworkManager {
    connection: Connection,
}

impl NetworkManager {
    pub fn system() -> Result<Self> {
        Ok(Self {
            connection: Connection::system().context("connect to the system D-Bus")?,
        })
    }

    fn proxy<'a>(&'a self, path: &'a str, iface: &'a str) -> Result<Proxy<'a>> {
        Proxy::new(&self.connection, NM, path, iface).context("create NetworkManager D-Bus proxy")
    }

    fn settings_map(&self, path: &str) -> Result<HashMap<String, HashMap<String, OwnedValue>>> {
        let p = self.proxy(path, CONNECTION_IFACE)?;
        let msg = p.call_method("GetSettings", &())?;
        msg.body()
            .deserialize()
            .context("decode saved NetworkManager profile")
    }
}

/// Block on NetworkManager D-Bus signals and notify the caller when its state
/// changes. The daemon uses this as its primary wake-up source and keeps a
/// slow timer as a recovery path for missed signals or service restarts.
pub fn monitor_events(tx: tokio::sync::mpsc::UnboundedSender<()>) -> Result<()> {
    let connection = Connection::system().context("connect to the system D-Bus for monitoring")?;
    let proxy =
        Proxy::new(&connection, NM, ROOT, NM).context("create NetworkManager signal monitor")?;
    let signals = proxy
        .receive_all_signals()
        .context("subscribe to NetworkManager D-Bus signals")?;
    for _signal in signals {
        if tx.send(()).is_err() {
            break;
        }
    }
    Ok(())
}

impl WifiBackend for NetworkManager {
    fn wifi_devices(&self) -> Result<Vec<WifiDevice>> {
        let p = self.proxy(ROOT, NM)?;
        let paths: Vec<OwnedObjectPath> = p
            .call("GetDevices", &())
            .context("list NetworkManager devices")?;
        let mut devices = Vec::new();
        for path in paths {
            let ty: u32 = {
                let device = self.proxy(path.as_str(), DEV_IFACE)?;
                device.get_property("DeviceType")?
            };
            if ty == 2 {
                devices.push(WifiDevice { path });
            }
        }
        devices.sort_by_key(|d| {
            self.proxy(d.path.as_str(), DEV_IFACE)
                .ok()
                .and_then(|proxy| {
                    proxy
                        .get_property::<OwnedObjectPath>("ActiveConnection")
                        .ok()
                })
                .map(|path| path.as_str() == "/")
                .unwrap_or(true)
        });
        Ok(devices)
    }

    fn saved_networks(&self) -> Result<Vec<ProfileDetails>> {
        let p = self.proxy(SETTINGS, SETTINGS_IFACE)?;
        let paths: Vec<OwnedObjectPath> = p
            .call("ListConnections", &())
            .context("list saved NetworkManager profiles")?;
        let mut profiles = Vec::new();
        for path in paths {
            let settings = match self.settings_map(path.as_str()) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let Some(wifi) = settings.get("802-11-wireless") else {
                continue;
            };
            let conn = settings
                .get("connection")
                .ok_or_else(|| anyhow!("profile lacks connection settings"))?;
            let uuid = value::<String>(conn.get("uuid"))?;
            let id = value::<String>(conn.get("id"))?;
            let ssid = value::<Vec<u8>>(wifi.get("ssid"))?;
            let security = if settings.contains_key("802-11-wireless-security") {
                "secured"
            } else {
                "open"
            }
            .to_string();
            profiles.push(ProfileDetails {
                core: SavedNetwork {
                    profile_uuid: uuid,
                    id,
                    ssid,
                },
                path,
                security,
            });
        }
        Ok(profiles)
    }

    fn access_points(
        &self,
        device: &WifiDevice,
        profiles: &[ProfileDetails],
        now: u64,
    ) -> Result<Vec<ApDetails>> {
        let wifi = self.proxy(device.path.as_str(), WIFI_IFACE)?;
        let paths: Vec<OwnedObjectPath> = wifi
            .get_property("AccessPoints")
            .context("read visible access points")?;
        let mut aps = Vec::new();
        let boot = boot_seconds();
        for path in paths {
            let p = self.proxy(path.as_str(), AP_IFACE)?;
            let ssid: Vec<u8> = p.get_property("Ssid").unwrap_or_default();
            let bssid: String = p.get_property("HwAddress").unwrap_or_default();
            let frequency: u32 = p.get_property("Frequency").unwrap_or_default();
            let strength: u8 = p.get_property("Strength").unwrap_or_default();
            let flags: u32 = p.get_property("Flags").unwrap_or_default();
            let last_seen: i32 = p.get_property("LastSeen").unwrap_or(-1);
            if bssid.is_empty() {
                continue;
            }
            // NetworkManager exposes AP strength as percentage. Keep it a
            // percentage-shaped quality value; never label it as dBm.
            let privacy = flags & 1 != 0;
            let matching: Vec<Option<String>> = profiles
                .iter()
                .filter(|p| p.core.ssid == ssid && (p.security != "open") == privacy)
                .map(|p| Some(p.core.profile_uuid.clone()))
                .collect();
            // NetworkManager reports LastSeen in CLOCK_BOOTTIME seconds. Convert
            // its age to the policy's wall-clock timestamp without persisting it.
            let age = boot
                .zip((last_seen >= 0).then_some(last_seen as u64))
                .map(|(boot, last)| boot.saturating_sub(last));
            let observed_at = age.map(|age| now.saturating_sub(age)).unwrap_or(0);
            // Duplicate the observation for each saved matching profile. This
            // preserves eligibility when users have multiple profiles for an SSID.
            for profile_uuid in if matching.is_empty() {
                vec![None]
            } else {
                matching
            } {
                aps.push(ApDetails {
                    core: AccessPoint {
                        profile_uuid,
                        ssid: ssid.clone(),
                        bssid: bssid.clone(),
                        frequency_mhz: (frequency != 0).then_some(frequency),
                        signal: Some(f32::from(strength)),
                        observed_at,
                    },
                    path: path.clone(),
                });
            }
        }
        Ok(aps)
    }

    fn current_connection(
        &self,
        device: &WifiDevice,
        profiles: &[ProfileDetails],
    ) -> Result<Option<CurrentConnection>> {
        let dev = self.proxy(device.path.as_str(), DEV_IFACE)?;
        let active: OwnedObjectPath = match dev.get_property::<OwnedObjectPath>("ActiveConnection")
        {
            Ok(p) if p.as_str() != "/" => p,
            _ => return Ok(None),
        };
        let ac = self.proxy(
            active.as_str(),
            "org.freedesktop.NetworkManager.Connection.Active",
        )?;
        let profile_path: OwnedObjectPath = ac.get_property("Connection")?;
        let profile = profiles.iter().find(|p| p.path == profile_path);
        let Some(profile) = profile else {
            return Ok(None);
        };
        let ap_path: OwnedObjectPath = self
            .proxy(device.path.as_str(), WIFI_IFACE)?
            .get_property("ActiveAccessPoint")?;
        if ap_path.as_str() == "/" {
            return Ok(None);
        }
        let ap = self.proxy(ap_path.as_str(), AP_IFACE)?;
        let bssid: String = ap.get_property("HwAddress")?;
        let frequency: u32 = ap.get_property("Frequency").unwrap_or_default();
        let strength: u8 = ap.get_property("Strength").unwrap_or_default();
        Ok(Some(CurrentConnection {
            profile_uuid: profile.core.profile_uuid.clone(),
            ssid: profile.core.ssid.clone(),
            bssid,
            frequency_mhz: (frequency != 0).then_some(frequency),
            signal: Some(f32::from(strength)),
        }))
    }

    fn request_scan(&self, device: &WifiDevice) -> Result<()> {
        self.proxy(device.path.as_str(), WIFI_IFACE)?
            .call::<_, _, ()>("RequestScan", &(HashMap::<String, OwnedValue>::new(),))
            .context("request Wi-Fi scan from NetworkManager")
    }

    fn activate(
        &self,
        profile: &ProfileDetails,
        device: &WifiDevice,
        ap: Option<&ApDetails>,
    ) -> Result<()> {
        let specific = ap
            .map(|ap| ap.path.clone())
            .unwrap_or_else(|| OwnedObjectPath::try_from("/").expect("root D-Bus object path"));
        self.proxy(ROOT, NM)?
            .call::<_, _, OwnedObjectPath>(
                "ActivateConnection",
                &(&profile.path, &device.path, &specific),
            )
            .context("activate selected saved Wi-Fi profile")?;
        Ok(())
    }
}

fn value<T: TryFrom<OwnedValue>>(value: Option<&OwnedValue>) -> Result<T>
where
    <T as TryFrom<OwnedValue>>::Error: std::fmt::Display,
{
    let v = value.ok_or_else(|| anyhow!("required profile property is missing"))?;
    T::try_from(v.clone()).map_err(|e| anyhow!("invalid saved profile property: {e}"))
}

fn boot_seconds() -> Option<u64> {
    let uptime = std::fs::read_to_string("/proc/uptime").ok()?;
    uptime
        .split_whitespace()
        .next()?
        .parse::<f64>()
        .ok()
        .map(|seconds| seconds.max(0.0) as u64)
}
