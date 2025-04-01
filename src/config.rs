use getset::Getters;
use std::collections::HashMap;
use std::{env, net::SocketAddr};

#[derive(Clone, Getters, Debug)]
pub struct Config {
    #[get = "pub"]
    ldap_url: String,
    #[get = "pub"]
    bind_dn: String,
    #[get = "pub"]
    bind_password: String,
    #[get = "pub"]
    bind_addr: SocketAddr,
    #[get = "pub"]
    refresh_interval_secs: u64,
    #[get = "pub"]
    group_search_base: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Self::from_map(env::vars().collect())
            .map_err(|e| format!("Failed to load config from environment: {e}").into())
    }

    pub fn from_map(vars: HashMap<String, String>) -> Result<Self, Box<dyn std::error::Error>> {
        let get = |key: &str| -> Result<String, Box<dyn std::error::Error>> {
            vars.get(key)
                .cloned()
                .ok_or_else(|| format!("{key} is required").into())
        };

        let ldap_url = get("LDAP_URL")?;
        let bind_dn = get("LDAP_BIND_DN")?;
        let bind_password = get("LDAP_PASSWORD")?;

        let bind_addr = vars
            .get("BIND_ADDR")
            .cloned()
            .unwrap_or_else(|| "127.0.0.1:8080".to_string())
            .parse::<SocketAddr>()
            .map_err(|_| "Invalid BIND_ADDR")?;

        let refresh_interval_secs = vars
            .get("REFRESH_INTERVAL_SECS")
            .cloned()
            .unwrap_or_else(|| "180".to_string())
            .parse::<u64>()
            .map_err(|_| "Invalid REFRESH_INTERVAL_SECS")?;

        let group_search_base = get("GROUP_SEARCH_BASE")?;

        Ok(Config {
            ldap_url,
            bind_dn,
            bind_password,
            bind_addr,
            refresh_interval_secs,
            group_search_base,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn test_fails_with_missing_required_vars() {
        let vars = make_vars(&[
            ("BIND_ADDR", "0.0.0.0:9000"),
            ("REFRESH_INTERVAL_SECS", "99"),
        ]);

        let result = Config::from_map(vars);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("LDAP_URL"));
    }

    #[test]
    fn test_defaults_are_used() {
        let vars = make_vars(&[
            ("LDAP_URL", "ldaps://example.com"),
            ("LDAP_BIND_DN", "cn=admin"),
            ("LDAP_PASSWORD", "secret"),
            ("GROUP_SEARCH_BASE", "ou=users,dc=example,dc=com"),
        ]);

        let config = Config::from_map(vars).unwrap();
        assert_eq!(config.bind_addr().to_string(), "127.0.0.1:8080");
        assert_eq!(*config.refresh_interval_secs(), 180);
    }

    #[test]
    fn test_defaults_can_be_overridden() {
        let vars = make_vars(&[
            ("LDAP_URL", "ldaps://override.com"),
            ("LDAP_BIND_DN", "cn=admin,dc=override"),
            ("LDAP_PASSWORD", "supersecret"),
            ("GROUP_SEARCH_BASE", "ou=groups,dc=override"),
            ("BIND_ADDR", "0.0.0.0:9999"),
            ("REFRESH_INTERVAL_SECS", "45"),
        ]);

        let config = Config::from_map(vars).unwrap();
        assert_eq!(config.ldap_url(), "ldaps://override.com");
        assert_eq!(config.bind_addr().to_string(), "0.0.0.0:9999");
        assert_eq!(*config.refresh_interval_secs(), 45);
    }
}
