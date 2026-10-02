use anyhow::{Context, Result};
use roam_core::{
    Candidate, Config, CurrentConnection, Decision, Engine, EngineState, IpcRequest, IpcResponse,
    NetworkStatus, Snapshot, Status,
};
use roam_networkmanager::{NetworkManager, WifiBackend};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
    sync::{mpsc, oneshot, RwLock},
    time::{self, Duration},
};
use tracing::{info, warn};

enum Command {
    Configure(Config, oneshot::Sender<Result<()>>),
    Confirm(bool, oneshot::Sender<Result<()>>),
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .init();
    let mut config = load_config()?;
    let mut engine = Engine::new(config.clone());
    let backend = Arc::new(tokio::task::spawn_blocking(NetworkManager::system).await??);
    let socket = socket_path()?;
    if socket.exists() {
        fs::remove_file(&socket).context("remove stale local socket")?;
    }
    let listener = UnixListener::bind(&socket).context("create local Roam IPC socket")?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))
        .context("restrict local socket permissions")?;
    let initial = Arc::new(RwLock::new(Status {
        monitoring: true,
        state: "Searching".into(),
        mode: config.mode,
        responsiveness: config.responsiveness,
        current: None,
        candidate: None,
        eligible: vec![],
        error: None,
    }));
    let (tx, mut rx) = mpsc::channel::<Command>(16);
    tokio::spawn(serve(listener, tx, initial.clone()));
    info!("roaming service started");

    let mut ticker = time::interval(Duration::from_secs(5));
    let mut current_candidate = None;
    let mut activation: Option<(String, String, u64)> = None;
    let mut last_scan = 0u64;
    let mut activation_error: Option<String> = None;
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let worker_engine=std::mem::replace(&mut engine,Engine::new(config.clone()));
                let worker_backend=backend.clone();let worker_config=config.clone();
                let (updated_engine,evaluation)=tokio::task::spawn_blocking(move||{let mut engine=worker_engine;let evaluation=evaluate_once(&worker_backend,&mut engine,&worker_config);(engine,evaluation)}).await?;
                engine=updated_engine;
                match evaluation {
                    Ok((status, decision, device, aps, profiles, observed_current)) => {
                        let mut activation_finished=false;
                        if let Some((profile_uuid,bssid,deadline)) = activation.clone() {
                            if observed_current.as_ref().is_some_and(|c| c.profile_uuid == profile_uuid && c.bssid.eq_ignore_ascii_case(&bssid) && c.signal.is_some_and(|signal|signal>=35.0)) {
                                engine.activation_succeeded(unix_now()); activation=None; activation_finished=true;activation_error=None;
                            } else if unix_now() >= deadline {
                                engine.activation_failed(&profile_uuid,&bssid,unix_now()); activation=None; activation_finished=true;
                            }
                        }
                        if engine.state()==EngineState::Seeking && unix_now().saturating_sub(last_scan)>=15 {
                            if let Some(device)=device.as_ref(){let backend=backend.clone();let device=device.clone();let _=tokio::task::spawn_blocking(move||backend.request_scan(&device)).await;last_scan=unix_now();}
                        }
                        current_candidate = if activation_finished { None } else { match decision {
                            Decision::Roam(_) if activation.is_some() => None,
                            Decision::AskForConfirmation(_) if activation.is_some() => None,
                            Decision::Roam(candidate) => {
                                if let (Some(profile), Some(device), Some(ap)) = (profiles.iter().find(|p| p.core.profile_uuid == candidate.profile_uuid), device.as_ref(), aps.iter().find(|a| a.core.profile_uuid.as_deref() == Some(&candidate.profile_uuid) && a.core.bssid == candidate.bssid)) {
                                    let backend=backend.clone();let profile=profile.clone();let device=device.clone();let ap=ap.clone();
                                    match tokio::task::spawn_blocking(move||backend.activate(&profile,&device,Some(&ap))).await? {
                                        Ok(()) => { activation_error=None;activation=Some((candidate.profile_uuid.clone(),candidate.bssid.clone(),unix_now()+20)); None }
                                        Err(_) => { warn!("network activation failed");activation_error=Some("NetworkManager denied the connection change".into()); engine.activation_failed(&candidate.profile_uuid, &candidate.bssid, unix_now()); Some((candidate, String::new())) }
                                    }
                                } else { None }
                            }
                            Decision::AskForConfirmation(candidate) => Some((candidate, String::new())),
                            Decision::Stay { .. } => None,
                        }};
                        let mut status = status;
                        status.candidate = current_candidate.as_ref().and_then(|(c,_)| profiles.iter().find(|p| p.core.profile_uuid == c.profile_uuid).map(|p| NetworkStatus { profile_uuid: c.profile_uuid.clone(), name: network_name(&p.core.ssid,&p.core.id), signal: Some(c.filtered_signal.clamp(0.0,100.0) as u8), eligible:true,available:true }));
                        status.state = state_name(engine.state()).into();
                        status.error=activation_error.clone();
                        *initial.write().await = status;
                    }
                    Err(_) => { let mut s=initial.write().await; s.error=Some("NetworkManager is unavailable".into()); s.monitoring=false; }
                }
            }
            command = rx.recv() => match command {
                Some(Command::Configure(next, reply)) => {
                    config = next; engine.set_config(config.clone());
                    let result = save_config(&config);
                    if result.is_ok() {
                        current_candidate = None;
                        let mut status=initial.write().await;
                        status.mode=config.mode;status.responsiveness=config.responsiveness;status.candidate=None;
                        for network in &mut status.eligible {network.eligible=config.is_eligible(&network.profile_uuid);}
                        for id in &config.eligible_profiles {if !status.eligible.iter().any(|n|&n.profile_uuid==id){status.eligible.push(NetworkStatus{profile_uuid:id.clone(),name:id.clone(),signal:None,eligible:true,available:false});}}
                    }
                    let _ = reply.send(result);
                }
                Some(Command::Confirm(switch, reply)) => {
                    let pending = current_candidate.take();
                    let result = if pending.is_some() {
                        if !switch {engine.ignore_pending(unix_now());Ok(())}
                        else if let Some(confirmed)=engine.confirm(true,unix_now()) {
                            let target=confirmed.clone();let backend=backend.clone();
                            let result=tokio::task::spawn_blocking(move||activate_candidate(&backend,&confirmed)).await.map_err(|_|anyhow::anyhow!("NetworkManager operation failed"))?;
                            if result.is_ok(){activation=Some((target.profile_uuid,target.bssid,unix_now()+20));}
                            result
                        } else {Err(anyhow::anyhow!("The candidate is no longer available"))}
                    } else {Err(anyhow::anyhow!("There is no pending candidate"))};
                    if switch && result.is_err() { if let Some((candidate,_))=pending.as_ref() { engine.activation_failed(&candidate.profile_uuid,&candidate.bssid,unix_now()); } }
                    if switch {activation_error=if result.is_err(){Some("NetworkManager denied the connection change".into())}else{None};}
                    let _ = reply.send(result);
                }
                None => break,
            }
        }
    }
    let _ = fs::remove_file(socket);
    Ok(())
}

fn evaluate_once(
    backend: &NetworkManager,
    engine: &mut Engine,
    config: &Config,
) -> Result<(
    Status,
    Decision,
    Option<roam_networkmanager::WifiDevice>,
    Vec<roam_networkmanager::ApDetails>,
    Vec<roam_networkmanager::ProfileDetails>,
    Option<CurrentConnection>,
)> {
    let devices = backend.wifi_devices()?;
    let profiles = backend.saved_networks()?;
    let device = devices.first().cloned();
    let now = unix_now();
    let (current, aps) = if let Some(ref d) = device {
        (
            backend.current_connection(d, &profiles)?,
            backend.access_points(d, &profiles, now)?,
        )
    } else {
        (None, vec![])
    };
    let snapshot = Snapshot {
        now,
        current: current.clone(),
        access_points: aps.iter().map(|a| a.core.clone()).collect(),
    };
    let decision = engine.evaluate(&snapshot);
    let current_status = current.as_ref().and_then(|c| {
        profiles
            .iter()
            .find(|p| p.core.profile_uuid == c.profile_uuid)
            .map(|p| NetworkStatus {
                profile_uuid: c.profile_uuid.clone(),
                name: network_name(&p.core.ssid, &p.core.id),
                signal: c.signal.map(|v| v.clamp(0.0, 100.0) as u8),
                eligible: config.is_eligible(&c.profile_uuid),
                available: true,
            })
    });
    let mut eligible: Vec<NetworkStatus> = profiles
        .iter()
        .map(|p| NetworkStatus {
            profile_uuid: p.core.profile_uuid.clone(),
            name: network_name(&p.core.ssid, &p.core.id),
            signal: None,
            eligible: config.is_eligible(&p.core.profile_uuid),
            available: true,
        })
        .collect();
    eligible.extend(
        config
            .eligible_profiles
            .iter()
            .filter(|id| !profiles.iter().any(|p| &p.core.profile_uuid == *id))
            .map(|id| NetworkStatus {
                profile_uuid: id.clone(),
                name: id.clone(),
                signal: None,
                eligible: true,
                available: false,
            }),
    );
    let status = Status {
        monitoring: true,
        state: state_name(engine.state()).into(),
        mode: config.mode,
        responsiveness: config.responsiveness,
        current: current_status,
        candidate: None,
        eligible,
        error: None,
    };
    Ok((status, decision, device, aps, profiles, current))
}

fn activate_candidate(backend: &NetworkManager, candidate: &Candidate) -> Result<()> {
    let devices = backend.wifi_devices()?;
    let profiles = backend.saved_networks()?;
    let device = devices
        .first()
        .ok_or_else(|| anyhow::anyhow!("no Wi-Fi device is available"))?;
    let profile = profiles
        .iter()
        .find(|p| p.core.profile_uuid == candidate.profile_uuid)
        .ok_or_else(|| anyhow::anyhow!("saved profile is unavailable"))?;
    let aps = backend.access_points(device, &profiles, unix_now())?;
    let ap = aps
        .iter()
        .find(|a| {
            a.core.profile_uuid.as_deref() == Some(candidate.profile_uuid.as_str())
                && a.core.bssid == candidate.bssid
        })
        .ok_or_else(|| anyhow::anyhow!("candidate access point is no longer visible"))?;
    backend.activate(profile, device, Some(ap))
}

async fn serve(listener: UnixListener, tx: mpsc::Sender<Command>, status: Arc<RwLock<Status>>) {
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let tx = tx.clone();
        let status = status.clone();
        tokio::spawn(async move {
            let (read, mut write) = stream.into_split();
            let mut line = String::new();
            let mut reader = BufReader::new(read);
            let response = if reader.read_line(&mut line).await.is_ok() {
                match serde_json::from_str::<IpcRequest>(&line) {
                    Ok(IpcRequest::Status) => IpcResponse::Status {
                        status: status.read().await.clone(),
                    },
                    Ok(IpcRequest::Configure { config }) => {
                        let (reply, rx) = oneshot::channel();
                        if tx.send(Command::Configure(config, reply)).await.is_ok() {
                            match rx.await {
                                Ok(Ok(())) => IpcResponse::Ok,
                                Ok(Err(e)) => IpcResponse::Error {
                                    message: e.to_string(),
                                },
                                _ => IpcResponse::Error {
                                    message: "service unavailable".into(),
                                },
                            }
                        } else {
                            IpcResponse::Error {
                                message: "service unavailable".into(),
                            }
                        }
                    }
                    Ok(IpcRequest::Confirm { switch }) => {
                        let (reply, rx) = oneshot::channel();
                        if tx.send(Command::Confirm(switch, reply)).await.is_ok() {
                            match rx.await {
                                Ok(Ok(())) => IpcResponse::Ok,
                                Ok(Err(e)) => IpcResponse::Error {
                                    message: e.to_string(),
                                },
                                _ => IpcResponse::Error {
                                    message: "service unavailable".into(),
                                },
                            }
                        } else {
                            IpcResponse::Error {
                                message: "service unavailable".into(),
                            }
                        }
                    }
                    Err(_) => IpcResponse::Error {
                        message: "invalid local request".into(),
                    },
                }
            } else {
                IpcResponse::Error {
                    message: "empty local request".into(),
                }
            };
            if let Ok(mut json) = serde_json::to_vec(&response) {
                json.push(b'\n');
                let _ = write.write_all(&json).await;
            }
        });
    }
}

fn load_config() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        let config = Config::default();
        save_config(&config)?;
        return Ok(config);
    }
    let data = fs::read_to_string(path)?;
    Ok(toml::from_str(&data).context("parse Roam configuration")?)
}
fn save_config(config: &Config) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }
    let data = toml::to_string(config)?;
    fs::write(&path, data)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}
fn config_path() -> Result<PathBuf> {
    let base = dirs::config_dir().context("find XDG config directory")?;
    Ok(base.join("roam/config.toml"))
}
fn socket_path() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .context("XDG_RUNTIME_DIR is not set")?;
    Ok(base.join("roam.sock"))
}
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn state_name(state: EngineState) -> &'static str {
    match state {
        EngineState::Stable => "Monitoring",
        EngineState::Degrading => "Weak",
        EngineState::Seeking => "Searching",
        EngineState::CandidateFound => "Searching",
        EngineState::AwaitingConfirmation => "Confirm",
        EngineState::Roaming => "Switching",
        EngineState::Cooldown => "Monitoring",
    }
}
fn network_name(ssid: &[u8], profile_id: &str) -> String {
    if ssid.is_empty() {
        profile_id.to_owned()
    } else {
        String::from_utf8_lossy(ssid).into_owned()
    }
}
