use adw::prelude::*;
use roam_core::{Config, IpcRequest, IpcResponse, Mode, NetworkStatus, Responsiveness};
use std::{
    cell::{Cell, RefCell},
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    process::{Command, Stdio},
    rc::Rc,
};

fn main() -> glib::ExitCode {
    start_roaming_service();
    adw::init().expect("initialize libadwaita");
    let app = adw::Application::builder()
        .application_id("org.roam.WifiRoaming")
        .build();
    app.connect_activate(build_window);
    app.run()
}

fn start_roaming_service() {
    // Package installation enables this user unit for login. Starting it here
    // also makes the first GUI launch work in the current session, including
    // immediately after installing a package while already logged in.
    let _ = Command::new("systemctl")
        .args(["--user", "--no-block", "start", "roam.service"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn build_window(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Roam")
        .default_width(390)
        .default_height(700)
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 14);
    root.set_margin_top(22);
    root.set_margin_bottom(22);
    root.set_margin_start(22);
    root.set_margin_end(22);
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let heading = gtk::Label::new(Some("Roam"));
    heading.add_css_class("title-1");
    heading.set_xalign(0.0);
    heading.set_hexpand(true);
    header.append(&heading);
    let close_button = gtk::Button::new();
    close_button.set_child(Some(&gtk::Image::from_icon_name("window-close-symbolic")));
    close_button.set_tooltip_text(Some("Close window"));
    close_button.add_css_class("flat");
    let close_window = window.clone();
    close_button.connect_clicked(move |_| close_window.close());
    header.append(&close_button);
    root.append(&header);
    let active = gtk::Label::new(Some("Checking monitoring service…"));
    active.set_xalign(0.0);
    active.set_wrap(true);
    root.append(&active);

    let mode_label = gtk::Label::new(Some("_Mode"));
    mode_label.set_use_underline(true);
    mode_label.set_xalign(0.0);
    mode_label.add_css_class("heading");
    root.append(&mode_label);
    let mode = gtk::DropDown::from_strings(&["Automatic", "Confirm"]);
    mode_label.set_mnemonic_widget(Some(&mode));
    mode.set_hexpand(true);
    root.append(&mode);
    let response_label = gtk::Label::new(Some("_Responsiveness"));
    response_label.set_use_underline(true);
    response_label.set_xalign(0.0);
    response_label.add_css_class("heading");
    root.append(&response_label);
    let responsiveness = gtk::DropDown::from_strings(&["Low", "Medium", "High"]);
    response_label.set_mnemonic_widget(Some(&responsiveness));
    root.append(&responsiveness);

    let eligible_title = gtk::Label::new(Some("Networks eligible for roaming"));
    eligible_title.set_xalign(0.0);
    eligible_title.add_css_class("heading");
    root.append(&eligible_title);
    let networks = gtk::Box::new(gtk::Orientation::Vertical, 6);
    networks.set_margin_top(8);
    networks.set_margin_bottom(8);
    networks.set_margin_start(8);
    networks.set_margin_end(8);
    let network_scroll = gtk::ScrolledWindow::new();
    network_scroll.set_min_content_height(240);
    network_scroll.set_max_content_height(360);
    network_scroll.set_vexpand(true);
    network_scroll.set_has_frame(true);
    network_scroll.set_child(Some(&networks));
    root.append(&network_scroll);
    let current = gtk::Label::new(Some("Current\nNot connected"));
    current.set_xalign(0.0);
    current.set_wrap(true);
    root.append(&current);
    let candidate = gtk::Label::new(Some("Candidate\nNone"));
    candidate.set_xalign(0.0);
    candidate.set_wrap(true);
    root.append(&candidate);
    window.set_content(Some(&root));

    let config = Rc::new(RefCell::new(Config::default()));
    let socket = socket_path();
    let selected = Rc::new(RefCell::new(Vec::<(String, gtk::CheckButton)>::new()));
    let syncing = Rc::new(Cell::new(false));
    let conf = config.clone();
    let s = socket.clone();
    let syncing_mode = syncing.clone();
    mode.connect_selected_notify(move |d| {
        if syncing_mode.get() {
            return;
        }
        conf.borrow_mut().mode = if d.selected() == 1 {
            Mode::Confirm
        } else {
            Mode::Automatic
        };
        send(
            &s,
            IpcRequest::Configure {
                config: conf.borrow().clone(),
            },
        );
    });
    let conf = config.clone();
    let s = socket.clone();
    let syncing_resp = syncing.clone();
    responsiveness.connect_selected_notify(move |d| {
        if syncing_resp.get() {
            return;
        }
        conf.borrow_mut().responsiveness = match d.selected() {
            0 => Responsiveness::Low,
            2 => Responsiveness::High,
            _ => Responsiveness::Medium,
        };
        send(
            &s,
            IpcRequest::Configure {
                config: conf.borrow().clone(),
            },
        );
    });
    let status_label = active.clone();
    let current_label = current.clone();
    let candidate_label = candidate.clone();
    let mode_ui = mode.clone();
    let resp_ui = responsiveness.clone();
    let net_box = networks.clone();
    let selected_ui = selected.clone();
    let conf = config.clone();
    let socket_ui = socket.clone();
    let prompted = Rc::new(RefCell::new(None::<String>));
    let prompted_ui = prompted.clone();
    let parent = window.clone();
    let syncing_ui = syncing.clone();
    glib::timeout_add_seconds_local(3, move || {
        let Some(IpcResponse::Status { status }) = send(&socket_ui, IpcRequest::Status) else {
            status_label.set_text("Monitoring service is not running");
            current_label.set_text("Current\nUnavailable");
            return glib::ControlFlow::Continue;
        };
        let monitoring = if status.error.is_some() {
            "Could not switch Wi-Fi connection. Check NetworkManager or Polkit permissions."
                .to_owned()
        } else if status.monitoring {
            format!("● Monitoring active · {}", status.state)
        } else {
            "Monitoring is waiting for NetworkManager".to_owned()
        };
        status_label.set_text(&monitoring);
        current_label.set_text(&format!(
            "Current\n{} · {}",
            status
                .current
                .as_ref()
                .map(|n| n.name.as_str())
                .unwrap_or("Not connected"),
            status
                .current
                .as_ref()
                .map(signal_summary)
                .unwrap_or_else(|| "No signal".into())
        ));
        if let Some(c) = status.candidate.as_ref() {
            candidate_label.set_text(&format!(
                "Better candidate\n{} · {}",
                c.name,
                signal_summary(c)
            ));
            let key = format!("{}:{}", c.profile_uuid, c.name);
            if status.mode == Mode::Confirm && prompted_ui.borrow().as_deref() != Some(key.as_str())
            {
                *prompted_ui.borrow_mut() = Some(key);
                let current_name = status
                    .current
                    .as_ref()
                    .map(|n| n.name.as_str())
                    .unwrap_or("Unknown network");
                let dialog = adw::AlertDialog::new(
                    Some("Wi-Fi signal is degrading."),
                    Some(&format!(
                        "Current\n{} · {}\n\nBetter candidate\n{} · {}",
                        current_name,
                        status
                            .current
                            .as_ref()
                            .map(signal_summary)
                            .unwrap_or_else(|| "No signal".into()),
                        c.name,
                        signal_summary(c)
                    )),
                );
                dialog.add_response("ignore", "Ignore");
                dialog.add_response("switch", "Switch");
                dialog.set_default_response(Some("switch"));
                dialog.set_close_response("ignore");
                let socket = socket_ui.clone();
                dialog.choose(
                    Some(&parent),
                    None::<&adw::gio::Cancellable>,
                    move |response| {
                        let _ = send(
                            &socket,
                            IpcRequest::Confirm {
                                switch: response == "switch",
                            },
                        );
                    },
                );
            }
        } else {
            candidate_label.set_text("Candidate\nNone");
            *prompted_ui.borrow_mut() = None;
        }
        syncing_ui.set(true);
        mode_ui.set_selected(if status.mode == Mode::Confirm { 1 } else { 0 });
        resp_ui.set_selected(match status.responsiveness {
            Responsiveness::Low => 0,
            Responsiveness::Medium => 1,
            Responsiveness::High => 2,
        });
        {
            let mut c = conf.borrow_mut();
            c.mode = status.mode;
            c.responsiveness = status.responsiveness;
            c.eligible_profiles = status
                .eligible
                .iter()
                .filter(|n| n.eligible)
                .map(|n| n.profile_uuid.clone())
                .collect();
        }
        syncing_ui.set(false);
        for item in status.eligible.iter() {
            let existing = selected_ui
                .borrow()
                .iter()
                .find(|(uuid, _)| uuid == &item.profile_uuid)
                .map(|(_, check)| check.clone());
            if let Some(check) = existing {
                check.set_label(Some(&if item.available {
                    item.name.clone()
                } else {
                    format!("{} (unavailable)", item.name)
                }));
                check.set_sensitive(item.available);
                check.set_active(item.eligible);
            } else {
                let check = gtk::CheckButton::with_label(&if item.available {
                    item.name.clone()
                } else {
                    format!("{} (unavailable)", item.name)
                });
                check.set_active(item.eligible);
                check.set_sensitive(item.available);
                let uuid = item.profile_uuid.clone();
                let conf = conf.clone();
                let socket = socket_ui.clone();
                check.connect_toggled(move |b| {
                    let mut c = conf.borrow_mut();
                    c.eligible_profiles.retain(|id| id != &uuid);
                    if b.is_active() {
                        c.eligible_profiles.push(uuid.clone());
                    }
                    let _ = send(&socket, IpcRequest::Configure { config: c.clone() });
                });
                net_box.append(&check);
                selected_ui
                    .borrow_mut()
                    .push((item.profile_uuid.clone(), check));
            }
        }
        glib::ControlFlow::Continue
    });
    window.present();
}

fn signal_name(network: &NetworkStatus) -> &'static str {
    match network.signal.unwrap_or(0) {
        70..=100 => "Strong",
        50..=69 => "Good",
        30..=49 => "Weak",
        1..=29 => "Very weak",
        _ => "Searching",
    }
}
fn signal_summary(network: &NetworkStatus) -> String {
    network
        .signal
        .map(|signal| format!("{} ({}%)", signal_name(network), signal))
        .unwrap_or_else(|| "Searching".into())
}
fn socket_path() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("roam.sock")
}
fn send(path: &PathBuf, request: IpcRequest) -> Option<IpcResponse> {
    let mut stream = UnixStream::connect(path).ok()?;
    let mut bytes = serde_json::to_vec(&request).ok()?;
    bytes.push(b'\n');
    stream.write_all(&bytes).ok()?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .ok()?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).ok()?;
    serde_json::from_str(&line).ok()
}
