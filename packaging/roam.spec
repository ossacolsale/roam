Name:           roam
Version:        0.1.0
Release:        2%{?dist}
Summary:        Local Wi-Fi roaming assistant
License:        MIT
URL:            https://github.com/ossacolsale/roam
Source0:        https://github.com/ossacolsale/roam/archive/refs/tags/v%{version}.tar.gz
BuildRequires:  cargo rust gtk4-devel libadwaita-devel gcc pkgconf-pkg-config
Requires:       NetworkManager
Requires:       systemd

%description
Local Linux Wi-Fi roaming assistant that can automatically switch between
selected saved NetworkManager Wi-Fi profiles when the current connection
becomes weak or unstable.

%prep
%autosetup -n roam-%{version}

%build
cargo build --release --workspace

%install
install -D -m755 target/release/roamd %{buildroot}%{_bindir}/roamd
install -D -m755 target/release/roam-ui %{buildroot}%{_bindir}/roam-ui
install -D -m644 systemd/roam.service %{buildroot}%{_userunitdir}/roam.service
mkdir -p %{buildroot}%{_userunitdir}/default.target.wants
ln -s ../roam.service %{buildroot}%{_userunitdir}/default.target.wants/roam.service
install -D -m644 packaging/org.roam.WifiRoaming.desktop %{buildroot}%{_datadir}/applications/org.roam.WifiRoaming.desktop
install -D -m644 packaging/org.roam.WifiRoaming.metainfo.xml %{buildroot}%{_datadir}/metainfo/org.roam.WifiRoaming.metainfo.xml
install -D -m644 assets/icons/hicolor/scalable/apps/org.roam.WifiRoaming.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/org.roam.WifiRoaming.svg
for size in 16 24 32 48 64 128 256 512; do \
  install -D -m644 assets/icons/hicolor/${size}x${size}/apps/org.roam.WifiRoaming.png %{buildroot}%{_datadir}/icons/hicolor/${size}x${size}/apps/org.roam.WifiRoaming.png; \
done

%files
%{_bindir}/roamd
%{_bindir}/roam-ui
%{_userunitdir}/roam.service
%{_userunitdir}/default.target.wants/roam.service
%{_datadir}/applications/org.roam.WifiRoaming.desktop
%{_datadir}/metainfo/org.roam.WifiRoaming.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/org.roam.WifiRoaming.svg
%{_datadir}/icons/hicolor/*/apps/org.roam.WifiRoaming.png
%license LICENSE
%doc README.md SECURITY.md
