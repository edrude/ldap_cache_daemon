# ldap_cache_daemon

`ldap_cache_daemon` is a lightweight Rust-based HTTP service that performs LDAP group membership lookups and caches them in memory. It’s designed for environments where repeated LDAP queries for the same group membership can be optimized with short-term caching.

✅ **Prebuilt RPMs are available for Rocky Linux 8 and 9** — see the [Releases page](https://github.com/edrude/ldap_cache_daemon/releases).

---

## ❓ Why

The main goal of this project is to **minimize LDAP connection overhead** in systems where group membership information is frequently needed.

- 🔌 A **single LDAP connection** is used during each background refresh cycle, shared across all cached group updates.
- 🆕 A new LDAP connection is only created the **first time** an uncached group is requested.
- 🔄 After that, group data is **cached in memory**, and reused until the next refresh interval.

This design keeps LDAP traffic light, avoids frequent binds, and reduces load on upstream directory servers.

---

## 🔧 Configuration

The daemon is configured via environment variables, typically stored in `/etc/sysconfig/ldap_cache_daemon`.

This file is loaded automatically by the included `systemd` service unit.

### Example: `/etc/sysconfig/ldap_cache_daemon`

```ini
# Environment file for ldap_cache_daemon
# Place this file at /etc/sysconfig/ldap_cache_daemon
# It will be loaded by systemd and read by the application via `Config::from_env`

########################################
# General logging
########################################

# Sets the log verbosity for the daemon.
# Valid values: trace, debug, info, warn, error
# Default: info
# LDAP_CACHE_LOG_LEVEL=info


########################################
# LDAP server configuration
########################################

# URL of the LDAP server. Must include the scheme.
# e.g., ldap://ldap.example.com or ldaps://ldap.example.com:636
LDAP_URL=ldaps://ldap.example.com:636

# Distinguished Name (DN) used to bind to the LDAP server.
# This should have permission to search for group and user info.
LDAP_BIND_DN=cn=admin,dc=example,dc=com

# Password for the bind DN.
LDAP_PASSWORD=secret


########################################
# Group lookup configuration
########################################

# Base DN where group entries live.
# This will be used to construct queries like: cn=groupname,<base>
GROUP_SEARCH_BASE=ou=groups,dc=example,dc=com


########################################
# Server configuration
########################################

# Address and port the daemon listens on.
# Format: IP:PORT — e.g., 127.0.0.1:8080 or 0.0.0.0:9000
# Default: 127.0.0.1:8080
# BIND_ADDR=127.0.0.1:8080

# How often to refresh cached group data in seconds.
# Default: 180
# REFRESH_INTERVAL_SECS=180
```

---

## 📦 Installation

### Option 1: From GitHub Releases

Download the appropriate `.rpm` package from the [Releases](https://github.com/edrude/ldap_cache_daemon/releases) page:

```bash
# Example for Rocky Linux 9:
sudo dnf install ./ldap_cache_daemon-0.1.0-1.el9.x86_64.rpm
```

---

## 🚀 Usage

Enable and start the systemd service:

```bash
sudo systemctl enable --now ldap_cache_daemon
```

By default, the daemon listens on `127.0.0.1:8080`.

### Endpoint: `/group_members`

#### Request

```
GET /group_members?name=examplegroup
```

#### Response

```json
["uid1", "uid2", "uid3"]
```

Cached entries will automatically refresh after the configured interval.

---

## 🛠 Built With

- [Rust](https://www.rust-lang.org/)
- [Axum](https://docs.rs/axum)
- [ldap3](https://docs.rs/ldap3)
- [Systemd](https://www.freedesktop.org/wiki/Software/systemd/)

---

## 📜 License

This project is licensed under the GNU General Public License, version 2 or later (GPL-2.0-or-later).  
See the [LICENSE](./LICENSE) file for details.
