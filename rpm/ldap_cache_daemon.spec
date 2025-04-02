# binary is built outside of RPM building
%global debug_package %{nil}
%global _build_id_links none

Name:           ldap_cache_daemon
Version:        0.1.0
Release:        1%{?dist}
Summary:        Rust-based LDAP cache daemon

License:        MIT
URL:            https://github.com/edrude/ldap_cache_daemon
Source0:        %{name}.tar.gz

BuildArch:      x86_64

%description
A small Rust daemon that caches LDAP lookups.

%prep
%autosetup -n %{name}

%build
# Nothing to build here because we already built the binary.


%install
mkdir -p %{buildroot}/opt/ldap_cache_daemon/bin
cp -a opt/ldap_cache_daemon/bin/ldap_cache_daemon %{buildroot}/opt/ldap_cache_daemon/bin/

mkdir -p %{buildroot}/etc/sysconfig
cp -a etc/sysconfig/ldap_cache_daemon %{buildroot}/etc/sysconfig/

mkdir -p %{buildroot}/usr/lib/systemd/system
cp -a usr/lib/systemd/system/ldap_cache_daemon.service %{buildroot}/usr/lib/systemd/system/

%files
%dir /opt/ldap_cache_daemon
%dir /opt/ldap_cache_daemon/bin
/opt/ldap_cache_daemon/bin/ldap_cache_daemon

%config(noreplace) /etc/sysconfig/ldap_cache_daemon
/usr/lib/systemd/system/ldap_cache_daemon.service

%changelog
* Thu Apr 03 2025 Ed Rude <ed.rude@gmail.com> - 0.1.0-1
- Initial RPM build
