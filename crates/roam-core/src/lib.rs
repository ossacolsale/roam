//! Platform independent Wi-Fi roaming policy.
//!
//! The engine accepts snapshots from a backend and returns a decision. It never
//! performs I/O, stores identifiers on disk, or knows about GTK/NetworkManager.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

const HISTORY_LIMIT: usize = 8;
const SIGNAL_SAMPLE_INTERVAL_SECS: u64 = 5;
const MIN_RECOVERY_SAMPLES: usize = 3;
// NetworkManager's public AccessPoint API reports percentage strength, not
// dBm. Policy operates on that native 0..100 scale and never presents it as dBm.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Automatic,
    Confirm,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Responsiveness {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub responsiveness: Responsiveness,
    #[serde(default)]
    pub eligible_profiles: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Automatic,
            responsiveness: Responsiveness::Medium,
            eligible_profiles: Vec::new(),
        }
    }
}

impl Config {
    pub fn is_eligible(&self, profile_uuid: &str) -> bool {
        self.eligible_profiles.iter().any(|id| id == profile_uuid)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AccessPoint {
    pub profile_uuid: Option<String>,
    pub ssid: Vec<u8>,
    pub bssid: String,
    pub frequency_mhz: Option<u32>,
    /// Signal quality on the backend's native scale (NetworkManager: 0..100%).
    pub signal: Option<f32>,
    pub observed_at: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CurrentConnection {
    pub profile_uuid: String,
    pub ssid: Vec<u8>,
    pub bssid: String,
    pub frequency_mhz: Option<u32>,
    pub signal: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub now: u64,
    pub current: Option<CurrentConnection>,
    pub access_points: Vec<AccessPoint>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub profile_uuid: String,
    pub ssid: Vec<u8>,
    pub bssid: String,
    pub frequency_mhz: Option<u32>,
    pub filtered_signal: f32,
    /// Advantage in native signal quality points (NetworkManager percentage).
    pub advantage: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Decision {
    Stay {
        state: EngineState,
        candidate_count: usize,
        candidate: Option<Candidate>,
    },
    AskForConfirmation(Candidate),
    Roam(Candidate),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineState {
    Stable,
    Degrading,
    Seeking,
    CandidateFound,
    AwaitingConfirmation,
    Roaming,
    Cooldown,
}

#[derive(Clone, Debug)]
struct SignalTrack {
    history: VecDeque<f32>,
    consecutive: u8,
    last_seen: u64,
    first_seen: u64,
    last_sample_at: u64,
}

impl SignalTrack {
    fn add(&mut self, signal: f32, now: u64) {
        // The daemon also polls on a 10 second fallback interval. Keep the
        // same AP's stability history across that gap so an idle desktop can
        // still establish that a candidate is consistently good.
        let gap = now.saturating_sub(self.last_seen);
        if gap > 60 {
            self.history.clear();
            self.consecutive = 0;
            self.first_seen = now;
            self.last_sample_at = now;
        }
        self.last_seen = now;
        if gap > 60 || now.saturating_sub(self.last_sample_at) >= SIGNAL_SAMPLE_INTERVAL_SECS {
            self.history.push_back(signal);
            while self.history.len() > HISTORY_LIMIT {
                self.history.pop_front();
            }
            self.consecutive = self.consecutive.saturating_add(1);
            self.last_sample_at = now;
        }
    }

    fn estimate(&self) -> f32 {
        let mut samples: Vec<f32> = self.history.iter().copied().collect();
        samples.sort_by(f32::total_cmp);
        let middle = samples.len() / 2;
        if samples.len() & 1 == 0 {
            (samples[middle - 1] + samples[middle]) / 2.0
        } else {
            samples[middle]
        }
    }

    fn confirms_recovery(&self, threshold: f32) -> bool {
        self.history.len() >= MIN_RECOVERY_SAMPLES && self.estimate() > threshold
    }

    fn unstable(&self) -> bool {
        if self.history.len() < 2 {
            return false;
        }
        let min = self.history.iter().copied().fold(f32::INFINITY, f32::min);
        let max = self
            .history
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        max - min >= 18.0
    }
}

#[derive(Clone, Debug)]
struct Suppression {
    until: u64,
}

#[derive(Clone, Debug)]
struct Policy {
    advantage: f32,
    minimum_candidate_signal: f32,
    degraded_signal: f32,
    critical_signal: f32,
    confirmations: u8,
    cooldown_secs: u64,
    suppression_secs: u64,
    degraded_secs: u64,
    candidate_stable_secs: u64,
    recovery_secs: u64,
}

impl From<Responsiveness> for Policy {
    fn from(value: Responsiveness) -> Self {
        match value {
            Responsiveness::Low => Self {
                advantage: 15.0,
                minimum_candidate_signal: 60.0,
                degraded_signal: 30.0,
                critical_signal: 12.0,
                confirmations: 3,
                cooldown_secs: 90,
                suppression_secs: 300,
                degraded_secs: 30,
                candidate_stable_secs: 20,
                recovery_secs: 10,
            },
            Responsiveness::Medium => Self {
                advantage: 12.0,
                minimum_candidate_signal: 55.0,
                degraded_signal: 42.0,
                critical_signal: 18.0,
                confirmations: 2,
                cooldown_secs: 60,
                suppression_secs: 180,
                degraded_secs: 20,
                candidate_stable_secs: 12,
                recovery_secs: 10,
            },
            Responsiveness::High => Self {
                advantage: 9.0,
                minimum_candidate_signal: 50.0,
                degraded_signal: 49.0,
                critical_signal: 22.0,
                confirmations: 2,
                cooldown_secs: 30,
                suppression_secs: 120,
                degraded_secs: 12,
                candidate_stable_secs: 8,
                recovery_secs: 10,
            },
        }
    }
}

/// Stateful but entirely in-memory roaming decision engine.
pub struct Engine {
    config: Config,
    tracks: HashMap<String, SignalTrack>,
    cooldown_until: Option<u64>,
    suppressed: HashMap<String, Suppression>,
    failed_until: HashMap<String, u64>,
    pending: Option<Candidate>,
    state: EngineState,
    degraded_since: Option<u64>,
    degraded_key: Option<String>,
    healthy_since: Option<u64>,
}

impl Engine {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            tracks: HashMap::new(),
            cooldown_until: None,
            suppressed: HashMap::new(),
            failed_until: HashMap::new(),
            pending: None,
            state: EngineState::Stable,
            degraded_since: None,
            degraded_key: None,
            healthy_since: None,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
        self.pending = None;
        self.state = EngineState::Stable;
        self.degraded_since = None;
        self.degraded_key = None;
        self.healthy_since = None;
    }
    pub fn state(&self) -> EngineState {
        self.state
    }

    pub fn signal_estimate(&self, profile_uuid: &str, bssid: &str) -> Option<f32> {
        self.tracks
            .get(&key(profile_uuid, bssid))
            .map(SignalTrack::estimate)
    }

    /// Evaluate one observation. `eligible_profiles` is rechecked here, even if
    /// the backend has already filtered APs, so an unknown network cannot win.
    pub fn evaluate(&mut self, snapshot: &Snapshot) -> Decision {
        let policy = Policy::from(self.config.responsiveness);
        self.suppressed.retain(|_, v| v.until > snapshot.now);
        self.failed_until.retain(|_, until| *until > snapshot.now);
        self.tracks
            .retain(|_, track| snapshot.now.saturating_sub(track.last_seen) <= 600);

        let current_conn = snapshot.current.as_ref();
        let current_signal = current_conn
            .and_then(|c| {
                c.signal
                    .filter(|s| s.is_finite() && (0.0..=100.0).contains(s))
            })
            .map(|signal| {
                let c = current_conn.expect("signal must belong to a current connection");
                let k = key(&c.profile_uuid, &c.bssid);
                self.observe(&k, signal, snapshot.now);
                self.tracks[&k].estimate()
            });
        let current = current_signal.unwrap_or(0.0);
        let current_key = current_conn.map(|c| key(&c.profile_uuid, &c.bssid));
        if current_key != self.degraded_key {
            self.degraded_since = None;
            self.healthy_since = None;
            self.degraded_key = current_key.clone();
        }
        let current_unstable = current_key
            .as_ref()
            .and_then(|k| self.tracks.get(k))
            .is_some_and(SignalTrack::unstable);
        let current_degrading_trend = current_key
            .as_ref()
            .and_then(|k| self.tracks.get(k))
            .is_some_and(|t| is_degrading(&t.history));
        let degraded_now = current_signal.is_none()
            || current <= policy.degraded_signal
            || current_unstable
            || current_degrading_trend;
        if current_signal.is_none() || degraded_now {
            self.healthy_since = None;
            self.degraded_since.get_or_insert(snapshot.now);
        } else if current_key
            .as_ref()
            .and_then(|k| self.tracks.get(k))
            .is_some_and(|track| track.confirms_recovery(policy.degraded_signal + 8.0))
        {
            let healthy_since = *self.healthy_since.get_or_insert(snapshot.now);
            if snapshot.now.saturating_sub(healthy_since) >= policy.recovery_secs {
                self.degraded_since = None;
            }
        } else {
            self.healthy_since = None;
        }
        let degrading = self
            .degraded_since
            .is_some_and(|since| snapshot.now.saturating_sub(since) >= policy.degraded_secs);

        let mut candidates: Vec<Candidate> = Vec::new();
        for ap in &snapshot.access_points {
            let Some(profile) = ap.profile_uuid.as_deref() else {
                continue;
            };
            if !self.config.is_eligible(profile) {
                continue;
            }
            if snapshot.now.saturating_sub(ap.observed_at) > 30 {
                continue;
            }
            let Some(signal) = ap
                .signal
                .filter(|s| s.is_finite() && (0.0..=100.0).contains(s))
            else {
                continue;
            };
            if current_conn.is_some_and(|c| ap.bssid.eq_ignore_ascii_case(&c.bssid)) {
                continue;
            }
            let k = key(profile, &ap.bssid);
            self.observe(&k, signal, snapshot.now);
            let track = &self.tracks[&k];
            if track.unstable()
                || track.consecutive < policy.confirmations
                || snapshot.now.saturating_sub(track.first_seen) < policy.candidate_stable_secs
            {
                continue;
            }
            let filtered_signal = track.estimate();
            if filtered_signal < policy.minimum_candidate_signal {
                continue;
            }
            if self.suppressed.contains_key(&k) || self.failed_until.contains_key(&k) {
                continue;
            }
            let advantage = filtered_signal - current;
            if advantage < policy.advantage {
                continue;
            }
            candidates.push(Candidate {
                profile_uuid: profile.to_owned(),
                ssid: ap.ssid.clone(),
                bssid: ap.bssid.clone(),
                frequency_mhz: ap.frequency_mhz,
                filtered_signal,
                advantage,
            });
        }
        candidates.sort_by(|a, b| b.filtered_signal.total_cmp(&a.filtered_signal));
        let count = candidates.len();
        let Some(candidate) = candidates.into_iter().next() else {
            return self.stay(
                if degrading {
                    EngineState::Seeking
                } else if degraded_now {
                    EngineState::Degrading
                } else {
                    EngineState::Stable
                },
                count,
                None,
            );
        };

        let emergency =
            current <= policy.critical_signal && candidate.advantage >= policy.advantage + 8.0;
        if self
            .cooldown_until
            .is_some_and(|until| until > snapshot.now)
            && !emergency
        {
            return self.stay(EngineState::Cooldown, count, Some(candidate));
        }
        if !degrading && !emergency {
            return self.stay(EngineState::CandidateFound, count, Some(candidate));
        }
        self.pending = Some(candidate.clone());
        match self.config.mode {
            Mode::Confirm => {
                self.state = EngineState::AwaitingConfirmation;
                Decision::AskForConfirmation(candidate)
            }
            Mode::Automatic => {
                self.state = EngineState::Roaming;
                Decision::Roam(candidate)
            }
        }
    }

    /// Call after the UI accepts or declines a pending confirmation.
    pub fn confirm(&mut self, accept: bool, now: u64) -> Option<Candidate> {
        let candidate = self.pending.take()?;
        if !accept {
            let k = key(&candidate.profile_uuid, &candidate.bssid);
            self.suppressed.insert(
                k,
                Suppression {
                    until: now
                        .saturating_add(Policy::from(self.config.responsiveness).suppression_secs),
                },
            );
            self.state = EngineState::Seeking;
            return None;
        }
        self.state = EngineState::Roaming;
        Some(candidate)
    }

    pub fn activation_succeeded(&mut self, now: u64) {
        self.cooldown_until =
            Some(now.saturating_add(Policy::from(self.config.responsiveness).cooldown_secs));
        self.pending = None;
        self.state = EngineState::Cooldown;
        self.degraded_since = None;
        self.degraded_key = None;
        self.healthy_since = None;
        self.tracks.clear();
    }

    pub fn activation_failed(&mut self, profile_uuid: &str, bssid: &str, now: u64) {
        let key = key(profile_uuid, bssid);
        self.failed_until
            .insert(key.clone(), now.saturating_add(30));
        self.pending = None;
        self.state = EngineState::Seeking;
    }

    pub fn ignore_pending(&mut self, now: u64) {
        let _ = self.confirm(false, now);
    }

    fn observe(&mut self, k: &str, signal: f32, now: u64) {
        self.tracks
            .entry(k.to_owned())
            .and_modify(|t| t.add(signal, now))
            .or_insert_with(|| SignalTrack {
                history: VecDeque::from([signal]),
                consecutive: 1,
                last_seen: now,
                first_seen: now,
                last_sample_at: now,
            });
    }

    fn stay(&mut self, state: EngineState, count: usize, candidate: Option<Candidate>) -> Decision {
        self.pending = None;
        self.state = state;
        Decision::Stay {
            state,
            candidate_count: count,
            candidate,
        }
    }
}

fn key(profile: &str, bssid: &str) -> String {
    format!("{profile}\0{}", bssid.to_ascii_lowercase())
}

fn is_degrading(history: &VecDeque<f32>) -> bool {
    if history.len() < 3 {
        return false;
    }
    let tail: Vec<f32> = history.iter().rev().take(4).copied().collect();
    tail.len() >= 3 && tail.first().unwrap() - *tail.last().unwrap() <= -10.0
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("configuration is invalid: {0}")]
    Invalid(String),
}

/// Test helper implementing a predictable Wi-Fi observation/activation source.
pub struct FakeWifiBackend {
    pub current: Option<CurrentConnection>,
    pub profiles: Vec<SavedNetwork>,
    pub access_points: Vec<AccessPoint>,
    pub activation_result: Result<(), String>,
    pub scan_count: usize,
    pub activation_count: usize,
}

impl Default for FakeWifiBackend {
    fn default() -> Self {
        Self {
            current: None,
            profiles: Vec::new(),
            access_points: Vec::new(),
            activation_result: Ok(()),
            scan_count: 0,
            activation_count: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedNetwork {
    pub profile_uuid: String,
    pub id: String,
    pub ssid: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Status {
    pub monitoring: bool,
    pub state: String,
    pub mode: Mode,
    pub responsiveness: Responsiveness,
    pub current: Option<NetworkStatus>,
    pub candidate: Option<NetworkStatus>,
    pub eligible: Vec<NetworkStatus>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub profile_uuid: String,
    pub name: String,
    pub signal: Option<u8>,
    pub eligible: bool,
    pub available: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum IpcRequest {
    Status,
    Configure { config: Config },
    Confirm { switch: bool },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum IpcResponse {
    Status { status: Status },
    Ok,
    Error { message: String },
}

impl FakeWifiBackend {
    pub fn snapshot(&self, now: u64) -> Snapshot {
        Snapshot {
            now,
            current: self.current.clone(),
            access_points: self.access_points.clone(),
        }
    }
    pub fn request_scan(&mut self) {
        self.scan_count += 1;
    }
    pub fn set_current(&mut self, current: Option<CurrentConnection>) {
        self.current = current;
    }
    pub fn set_access_points(&mut self, access_points: Vec<AccessPoint>) {
        self.access_points = access_points;
    }
    pub fn lose_connection(&mut self) {
        self.current = None;
    }
    pub fn activate(&mut self, _profile_uuid: &str, _bssid: &str) -> Result<(), &str> {
        self.activation_count += 1;
        self.activation_result.as_ref().map_err(|e| e.as_str())?;
        let Some(ap) = self.access_points.iter().find(|ap| {
            ap.profile_uuid.as_deref() == Some(_profile_uuid)
                && ap.bssid.eq_ignore_ascii_case(_bssid)
        }) else {
            return Err("candidate not visible");
        };
        self.current = Some(CurrentConnection {
            profile_uuid: _profile_uuid.to_owned(),
            ssid: ap.ssid.clone(),
            bssid: ap.bssid.clone(),
            frequency_mhz: ap.frequency_mhz,
            signal: ap.signal,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn current(signal: f32) -> CurrentConnection {
        CurrentConnection {
            profile_uuid: "office".into(),
            ssid: b"Office".to_vec(),
            bssid: "00:00:00:00:00:01".into(),
            frequency_mhz: Some(5180),
            signal: Some(signal),
        }
    }
    fn ap(profile: Option<&str>, bssid: &str, signal: f32) -> AccessPoint {
        AccessPoint {
            profile_uuid: profile.map(str::to_owned),
            ssid: b"Lab".to_vec(),
            bssid: bssid.into(),
            frequency_mhz: Some(5180),
            signal: Some(signal),
            observed_at: 0,
        }
    }
    fn engine(mode: Mode) -> Engine {
        Engine::new(Config {
            mode,
            responsiveness: Responsiveness::Medium,
            eligible_profiles: vec!["lab".into()],
        })
    }
    fn eval(e: &mut Engine, now: u64, cur: f32, aps: Vec<AccessPoint>) -> Decision {
        let access_points = aps
            .into_iter()
            .map(|mut ap| {
                ap.observed_at = now;
                ap
            })
            .collect();
        e.evaluate(&Snapshot {
            now,
            current: Some(current(cur)),
            access_points,
        })
    }

    #[test]
    fn healthy_connection_stays() {
        let mut e = engine(Mode::Automatic);
        for (n, s) in [75., 72., 70., 68.].into_iter().enumerate() {
            assert!(matches!(
                eval(
                    &mut e,
                    n as u64,
                    s,
                    vec![ap(Some("lab"), "00:00:00:00:00:02", 90.)]
                ),
                Decision::Stay { .. }
            ));
        }
    }
    #[test]
    fn weak_current_and_stable_candidate_roams() {
        let mut e = engine(Mode::Automatic);
        let _ = eval(
            &mut e,
            0,
            50.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        let _ = eval(
            &mut e,
            12,
            38.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 76.)],
        );
        let _ = eval(
            &mut e,
            32,
            20.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        let decision = eval(
            &mut e,
            52,
            20.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 74.)],
        );
        assert!(matches!(decision, Decision::Roam(_)), "{decision:?}");
    }

    #[test]
    fn medium_roams_from_weak_range_after_dwell() {
        let mut e = engine(Mode::Automatic);
        assert!(matches!(
            eval(
                &mut e,
                0,
                40.,
                vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)]
            ),
            Decision::Stay { .. }
        ));
        assert!(matches!(
            eval(
                &mut e,
                12,
                40.,
                vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)]
            ),
            Decision::Stay {
                state: EngineState::CandidateFound,
                ..
            }
        ));
        assert!(matches!(
            eval(
                &mut e,
                20,
                40.,
                vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)]
            ),
            Decision::Roam(_)
        ));
    }

    #[test]
    fn brief_signal_recovery_does_not_restart_degraded_timer() {
        let mut e = engine(Mode::Automatic);
        eval(
            &mut e,
            0,
            42.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        eval(
            &mut e,
            12,
            51.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        let decision = eval(
            &mut e,
            20,
            42.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        assert!(matches!(decision, Decision::Roam(_)), "{decision:?}");
    }

    #[test]
    fn sustained_median_recovery_clears_degraded_timer() {
        let mut e = engine(Mode::Automatic);
        for now in [0, 5, 10, 15] {
            eval(
                &mut e,
                now,
                if now == 0 { 42. } else { 52. },
                vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
            );
        }
        let decision = eval(
            &mut e,
            20,
            52.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        assert!(matches!(
            decision,
            Decision::Stay {
                state: EngineState::CandidateFound,
                ..
            }
        ));
    }

    #[test]
    fn rapid_signal_updates_do_not_overweight_the_median() {
        let mut e = engine(Mode::Automatic);
        eval(&mut e, 0, 40., vec![]);
        eval(&mut e, 1, 100., vec![]);
        eval(&mut e, 2, 100., vec![]);
        eval(&mut e, 10, 40., vec![]);
        assert_eq!(e.signal_estimate("office", "00:00:00:00:00:01"), Some(40.));
    }
    #[test]
    fn small_advantage_stays() {
        let mut e = engine(Mode::Automatic);
        for n in 0..4 {
            assert!(matches!(
                eval(
                    &mut e,
                    n,
                    50.,
                    vec![ap(Some("lab"), "00:00:00:00:00:02", 48.)]
                ),
                Decision::Stay { .. }
            ));
        }
    }
    #[test]
    fn fluctuating_candidate_is_rejected() {
        let mut e = engine(Mode::Automatic);
        for (n, s) in [90., 15., 88., 17., 91.].into_iter().enumerate() {
            let d = eval(
                &mut e,
                n as u64 * SIGNAL_SAMPLE_INTERVAL_SECS,
                20.,
                vec![ap(Some("lab"), "00:00:00:00:00:02", s)],
            );
            assert!(!matches!(d, Decision::Roam(_)));
        }
    }
    #[test]
    fn unknown_network_is_ignored() {
        let mut e = engine(Mode::Automatic);
        for n in 0..4 {
            let d = eval(
                &mut e,
                n,
                20.,
                vec![
                    ap(None, "00:00:00:00:00:09", 99.),
                    ap(Some("lab"), "00:00:00:00:00:02", 30.),
                ],
            );
            assert!(!matches!(d, Decision::Roam(_)));
        }
    }
    #[test]
    fn different_ssid_eligible_network_is_candidate() {
        let mut e = engine(Mode::Automatic);
        eval(
            &mut e,
            0,
            20.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        assert!(matches!(
            eval(
                &mut e,
                20,
                20.,
                vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)]
            ),
            Decision::Roam(_)
        ));
    }
    #[test]
    fn same_ssid_other_bssid_is_candidate() {
        let mut e = Engine::new(Config {
            mode: Mode::Automatic,
            responsiveness: Responsiveness::Medium,
            eligible_profiles: vec!["office".into()],
        });
        let mut same = ap(Some("office"), "00:00:00:00:00:02", 80.);
        same.ssid = b"Office".to_vec();
        for n in [0, 20] {
            let d = eval(&mut e, n, 20., vec![same.clone()]);
            if n == 20 {
                assert!(matches!(d, Decision::Roam(_)));
            }
        }
    }
    #[test]
    fn confirm_mode_never_roams_without_confirmation() {
        let mut e = engine(Mode::Confirm);
        eval(
            &mut e,
            0,
            20.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        let d = eval(
            &mut e,
            20,
            20.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        assert!(matches!(d, Decision::AskForConfirmation(_)));
        let accepted = e.confirm(true, 20);
        assert!(accepted.is_some());
    }
    #[test]
    fn cooldown_prevents_ping_pong() {
        let mut e = engine(Mode::Automatic);
        eval(
            &mut e,
            0,
            20.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        assert!(matches!(
            eval(
                &mut e,
                20,
                20.,
                vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)]
            ),
            Decision::Roam(_)
        ));
        e.activation_succeeded(20);
        let _ = eval(
            &mut e,
            21,
            20.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        let d = eval(
            &mut e,
            33,
            20.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        assert!(matches!(
            d,
            Decision::Stay {
                state: EngineState::Cooldown,
                ..
            }
        ));
    }
    #[test]
    fn failure_is_penalized_without_panic() {
        let mut e = engine(Mode::Automatic);
        let _ = eval(
            &mut e,
            0,
            20.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        let d = eval(
            &mut e,
            20,
            20.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        assert!(matches!(d, Decision::Roam(_)));
        e.activation_failed("lab", "00:00:00:00:00:02", 21);
        assert!(!matches!(
            eval(
                &mut e,
                22,
                20.,
                vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)]
            ),
            Decision::Roam(_)
        ));
    }
    #[test]
    fn config_defaults_and_roundtrips() {
        let c = Config::default();
        assert_eq!(c.mode, Mode::Automatic);
        assert_eq!(c.responsiveness, Responsiveness::Medium);
        let encoded = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<Config>(&encoded).unwrap(), c);
    }
    #[test]
    fn ignored_candidate_is_suppressed() {
        let mut e = engine(Mode::Confirm);
        eval(
            &mut e,
            0,
            20.,
            vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
        );
        assert!(matches!(
            eval(
                &mut e,
                20,
                20.,
                vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)]
            ),
            Decision::AskForConfirmation(_)
        ));
        e.ignore_pending(20);
        assert!(matches!(
            eval(
                &mut e,
                21,
                20.,
                vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)]
            ),
            Decision::Stay { .. }
        ));
    }
    #[test]
    fn high_responsiveness_roams_earlier_than_medium() {
        let mut high = Engine::new(Config {
            mode: Mode::Automatic,
            responsiveness: Responsiveness::High,
            eligible_profiles: vec!["lab".into()],
        });
        let mut medium = engine(Mode::Automatic);
        for n in [0, 8] {
            eval(
                &mut high,
                n,
                40.,
                vec![ap(Some("lab"), "00:00:00:00:00:02", 62.)],
            );
            eval(
                &mut medium,
                n,
                40.,
                vec![ap(Some("lab"), "00:00:00:00:00:02", 62.)],
            );
        }
        assert!(matches!(
            eval(
                &mut high,
                12,
                40.,
                vec![ap(Some("lab"), "00:00:00:00:00:02", 62.)]
            ),
            Decision::Roam(_)
        ));
        assert!(matches!(
            eval(
                &mut medium,
                12,
                40.,
                vec![ap(Some("lab"), "00:00:00:00:00:02", 62.)]
            ),
            Decision::Stay { .. }
        ));
    }
    #[test]
    fn fake_backend_scans_and_preserves_connection_on_failed_activation() {
        let old = current(20.);
        let mut fake = FakeWifiBackend {
            current: Some(old.clone()),
            profiles: vec![SavedNetwork {
                profile_uuid: "lab".into(),
                id: "Lab".into(),
                ssid: b"Lab".to_vec(),
            }],
            access_points: vec![ap(Some("lab"), "00:00:00:00:00:02", 75.)],
            ..Default::default()
        };
        fake.request_scan();
        assert_eq!(fake.scan_count, 1);
        fake.activation_result = Err("denied".into());
        assert!(fake.activate("lab", "00:00:00:00:00:02").is_err());
        assert_eq!(fake.current, Some(old));
        fake.activation_result = Ok(());
        assert!(fake.activate("lab", "00:00:00:00:00:02").is_ok());
        assert_eq!(fake.current.unwrap().profile_uuid, "lab");
        assert_eq!(fake.activation_count, 2);
    }
    #[test]
    fn connection_loss_still_finds_a_usable_eligible_candidate() {
        let mut e = engine(Mode::Automatic);
        let snapshot = |now| Snapshot {
            now,
            current: None,
            access_points: vec![AccessPoint {
                observed_at: now,
                ..ap(Some("lab"), "00:00:00:00:00:02", 70.)
            }],
        };
        assert!(matches!(e.evaluate(&snapshot(0)), Decision::Stay { .. }));
        assert!(matches!(e.evaluate(&snapshot(20)), Decision::Roam(_)));
    }
}
