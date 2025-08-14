# ldap_cache_daemon

`ldap_cache_daemon` is a lightweight Rust-based HTTP service that performs LDAP lookups and caches them in memory. It's designed for environments where repeated LDAP queries can be optimized with short-term caching and supports multiple endpoint types with configurable result processing.

**Prebuilt RPMs are available for Enterprise Linux 8 and 9** — see the [Releases page](https://github.com/edrude/ldap_cache_daemon/releases).

---

## Features

The main goal of this project is to **minimize LDAP connection overhead** in systems where group membership information is frequently needed.

This design keeps LDAP traffic light, avoids frequent binds, and reduces load on upstream directory servers.

---

## Configuration

The daemon is configured via a YAML configuration file located at `/opt/ldap_cache_daemon/etc/config.yaml`.

### Main Configuration: `/opt/ldap_cache_daemon/etc/config.yaml`

```yaml
ldap:
  url: "ldaps://ldap.example.com:636"
  bind_dn: "cn=admin,dc=example,dc=com"
  bind_password: "your_secure_password_here"

server:
  bind_addr: "127.0.0.1:8080"
  refresh_interval_secs: 180

endpoints:
  # Group membership endpoint with DN resolution
  - path: "/group_members"
    search_base: "ou=groups,dc=example,dc=com"
    search_filter: "(cn={})"
    search_scope: "subtree"
    attribute: "member"
    result_processing:
      type: "dn_translation"
      attribute: "uid"

  # User maildrop endpoint (no result processing)
  - path: "/user_maildrop"
    search_base: "ou=users,dc=example,dc=com"
    search_filter: "(uid={})"
    search_scope: "subtree"
    attribute: "maildrop"
```

### Configuration Options

#### LDAP Configuration
- `url`: LDAP server URL (ldap:// or ldaps://)
- `bind_dn`: Distinguished Name for LDAP binding
- `bind_password`: Password for the bind DN

#### Server Configuration
- `bind_addr`: IP address and port to bind to (e.g., "127.0.0.1:8080")
- `refresh_interval_secs`: How often to refresh cached data in seconds

#### Endpoint Configuration
- `path`: HTTP endpoint path (e.g., "/group_members")
- `search_base`: LDAP search base DN
- `search_filter`: LDAP search filter (use `{}` as placeholder for the name parameter)
- `search_scope`: LDAP search scope ("base", "one", "subtree")
- `attribute`: LDAP attribute to retrieve
- `result_processing`: Optional result processing configuration

#### Result Processing Types
- `dn_translation`: Resolves DNs to extract specific attributes
- `null`: No processing (raw results returned)

---

## Installation

### From GitHub Releases

Download the appropriate `.rpm` package from the [Releases](https://github.com/edrude/ldap_cache_daemon/releases) page:

```bash
# Example for Rocky Linux 9:
sudo dnf install ./ldap_cache_daemon-0.1.0-1.el9.x86_64.rpm
```

After installation, edit the configuration file at `/opt/ldap_cache_daemon/etc/config.yaml` with your LDAP server details.

---

## Security

The daemon includes built-in security checks to ensure your configuration file is properly protected:

### Config File Requirements

- **Ownership**: Must be owned by root (UID 0)
- **Permissions**: Must be 600 or more restrictive (only owner can read/write)
- **Validation**: The daemon will refuse to start if these requirements are not met

### Example Secure Setup

```bash
# Set proper ownership
sudo chown root:root /opt/ldap_cache_daemon/etc/config.yaml

# Set secure permissions (600 = owner read/write only)
sudo chmod 600 /opt/ldap_cache_daemon/etc/config.yaml
```

The RPM package automatically sets these permissions during installation.

### Development Bypass

For development and testing scenarios, you can bypass permission checks by setting:

```bash
export DONTBLAMEME=1
```

**Warning**: This bypasses security checks and should only be used in development environments, never in production.

---

## Usage

### Starting the Service

Enable and start the systemd service:

```bash
sudo systemctl enable --now ldap_cache_daemon
```

By default, the daemon listens on `127.0.0.1:8080`.

### API Endpoints

The daemon dynamically creates endpoints based on your configuration. Each endpoint follows the pattern:

```
GET /{endpoint_path}/{name}
```

#### Example: Group Membership

**Request:**
```bash
curl "http://127.0.0.1:8080/group_members/staff"
```

**Response:**
```json
["user1", "user2", "user3"]
```

#### Example: User Attributes

**Request:**
```bash
curl "http://127.0.0.1:8080/user_maildrop/user1"
```

**Response:**
```json
["john.doe@example.com"]
```

### Caching Behavior

- **First Request**: LDAP query is executed and result is cached
- **Subsequent Requests**: Cached result is returned immediately
- **Background Refresh**: Cache is automatically refreshed at the configured interval

---

## Development

### Building from Source

```bash
cargo build --release
```

---

## License

This project is licensed under the GNU General Public License, version 2 or later (GPL-2.0-or-later).  
See the [LICENSE](./LICENSE) file for details.
