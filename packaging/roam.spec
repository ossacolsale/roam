Name:           roam
Version:        0.1.0
Release:        1%{?dist}
Summary:        Local Wi-Fi roaming assistant
License:        MIT
BuildRequires:  cargo rust gtk4-devel libadwaita-devel gcc
Requires:       NetworkManager systemd

%description
GTK desktop application and user service for roaming among selected saved
NetworkManager Wi-Fi profiles.

%prep
%autosetup -n roam-%{version}

%build
cargo build --release --workspace

%install
install -D -m755 target/release/roamd %{buildroot}%{_bindir}/roamd
install -D -m755 target/release/roam-ui %{buildroot}%{_bindir}/roam-ui
install -D -m644 systemd/roam.service %{buildroot}%{_userunitdir}/roam.service
install -D -m644 packaging/org.roam.WifiRoaming.desktop %{buildroot}%{_datadir}/applications/org.roam.WifiRoaming.desktop

%files
%{_bindir}/roamd
%{_bindir}/roam-ui
%{_userunitdir}/roam.service
%{_datadir}/applications/org.roam.WifiRoaming.desktop
