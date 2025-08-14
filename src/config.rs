use getset::Getters;
use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr, fs, os::unix::fs::{PermissionsExt, MetadataExt}};
use log::error;

#[derive(Clone, Getters, Debug, Deserialize, Serialize)]
pub struct Config {
    #[get = "pub"]
    ldap: LdapConfig,
    #[get = "pub"]
    server: ServerConfig,
    #[get = "pub"]
    endpoints: Vec<EndpointConfig>,
}

#[derive(Clone, Getters, Debug, Deserialize, Serialize)]
pub struct LdapConfig {
    #[get = "pub"]
    url: String,
    #[get = "pub"]
    bind_dn: String,
    #[get = "pub"]
    bind_password: String,
}

impl LdapConfig {
    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate LDAP URL
        if self.url.is_empty() {
            return Err("LDAP URL cannot be empty".into());
        }
        
        if !self.url.starts_with("ldap://") && !self.url.starts_with("ldaps://") {
            return Err("LDAP URL must start with 'ldap://' or 'ldaps://'".into());
        }
        
        // Validate bind DN
        if self.bind_dn.is_empty() {
            return Err("LDAP bind DN cannot be empty".into());
        }
        
        // Validate bind password
        if self.bind_password.is_empty() {
            return Err("LDAP bind password cannot be empty".into());
        }
        
        Ok(())
    }
}

#[derive(Clone, Getters, Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    #[get = "pub"]
    bind_addr: SocketAddr,
    #[get = "pub"]
    refresh_interval_secs: u64,
}

impl ServerConfig {
    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate refresh interval
        if self.refresh_interval_secs == 0 {
            return Err("Refresh interval must be greater than 0 seconds".into());
        }
        
        if self.refresh_interval_secs > 86400 {
            return Err("Refresh interval cannot exceed 24 hours (86400 seconds)".into());
        }
        
        Ok(())
    }
}

#[derive(Clone, Getters, Debug, Deserialize, Serialize)]
pub struct EndpointConfig {
    #[get = "pub"]
    path: String,
    #[get = "pub"]
    search_base: String,
    #[get = "pub"]
    search_filter: String,
    #[get = "pub"]
    search_scope: String,
    #[get = "pub"]
    attribute: String,
    #[get = "pub"]
    result_processing: Option<ResultProcessing>,
}

impl EndpointConfig {
    fn validate(&self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        // Validate path
        if self.path.is_empty() {
            return Err(format!("Endpoint {}: path cannot be empty", index).into());
        }
        
        if !self.path.starts_with('/') {
            return Err(format!("Endpoint {}: path must start with '/'", index).into());
        }
        
        // Validate search base
        if self.search_base.is_empty() {
            return Err(format!("Endpoint {}: search_base cannot be empty", index).into());
        }
        
        // Validate search filter
        if self.search_filter.is_empty() {
            return Err(format!("Endpoint {}: search_filter cannot be empty", index).into());
        }
        
        if !self.search_filter.contains("{}") {
            return Err(format!("Endpoint {}: search_filter must contain '{{}}' placeholder", index).into());
        }
        
        // Validate search scope
        let valid_scopes = ["base", "one", "subtree"];
        if !valid_scopes.contains(&self.search_scope.as_str()) {
            return Err(format!("Endpoint {}: search_scope must be one of: {}", 
                index, valid_scopes.join(", ")).into());
        }
        
        // Validate attribute
        if self.attribute.is_empty() {
            return Err(format!("Endpoint {}: attribute cannot be empty", index).into());
        }
        
        // Validate result processing if present
        if let Some(processing) = &self.result_processing {
            processing.validate(index)?;
        }
        
        Ok(())
    }
}

#[derive(Clone, Getters, Debug, Deserialize, Serialize)]
pub struct ResultProcessing {
    #[get = "pub"]
    r#type: String, // `r#type` so it doesn't conflict with Rust's `type` keyword
    #[get = "pub"]
    attribute: String,
}

impl ResultProcessing {
    fn validate(&self, endpoint_index: usize) -> Result<(), Box<dyn std::error::Error>> {
        // Validate type
        let valid_types = ["dn_translation"];
        if !valid_types.contains(&self.r#type.as_str()) {
            return Err(format!("Endpoint {}: result_processing.type must be one of: {}", 
                endpoint_index, valid_types.join(", ")).into());
        }
        
        // Validate attribute
        if self.attribute.is_empty() {
            return Err(format!("Endpoint {}: result_processing.attribute cannot be empty", endpoint_index).into());
        }
        
        Ok(())
    }
}

impl Config {
    /// Check if the config file has secure permissions and ownership
    /// Only root should be able to read the file (600 or more restrictive)
    fn check_config_permissions(config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Check if permission checks should be skipped
        if env::var("DONTBLAMEME").unwrap_or_default() == "1" {
            log::warn!("DONTBLAMEME=1 set, skipping config file permission checks");
            return Ok(());
        }
        
        let metadata = fs::metadata(config_path)
            .map_err(|e| format!("Failed to get config file metadata: {e}"))?;
        
        let permissions = metadata.permissions();
        let mode = permissions.mode();
        
        // Check if others have read access (mode & 0o004 != 0)
        // Check if group has read access (mode & 0o040 != 0)
        // Only owner should have read access (600 = 0o600)
        if (mode & 0o077) != 0 {
            error!("Config file {} has insecure permissions: {:o}", config_path, mode);
            error!("File permissions must be 600 or more restrictive (only owner can read)");
            return Err("Config file has insecure permissions".into());
        }
        
        // Check if file is owned by root (UID 0)
        let uid = metadata.uid();
        if uid != 0 {
            error!("Config file {} is not owned by root (UID: {})", config_path, uid);
            error!("File must be owned by root for security");
            return Err("Config file is not owned by root".into());
        }
        
        Ok(())
    }

    /// Validate the entire configuration
    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate LDAP configuration
        self.ldap.validate()?;
        
        // Validate server configuration
        self.server.validate()?;
        
        // Validate endpoints
        if self.endpoints.is_empty() {
            return Err("No endpoints configured. At least one endpoint is required.".into());
        }
        
        for (i, endpoint) in self.endpoints.iter().enumerate() {
            endpoint.validate(i)?;
        }
        
        // Check for duplicate endpoint paths
        let mut paths = std::collections::HashSet::new();
        for endpoint in &self.endpoints {
            if !paths.insert(endpoint.path()) {
                return Err(format!("Duplicate endpoint path: {}", endpoint.path()).into());
            }
        }
        
        Ok(())
    }

    pub fn get_config() -> Result<Self, Box<dyn std::error::Error>> {
        let config_file = env::var("CONFIG_FILE").unwrap_or_else(|_| "/opt/ldap_cache_daemon/etc/config.yaml".to_string());

        // Check file permissions and ownership before reading
        Self::check_config_permissions(&config_file)?;

        let content = fs::read_to_string(config_file)
            .map_err(|e| format!("Failed to read config file: {e}"))?;

        let config: Config = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse YAML config: {e}"))?;

        // Validate the configuration
        config.validate()?;

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_check_config_permissions_secure() {
        // Create a temporary file with secure permissions (600)
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        
        // Set permissions to 600 (owner read/write only)
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms).unwrap();
        
        // Should pass (assuming running as non-root user)
        // Note: This test will fail if run as root, which is expected
        let result = Config::check_config_permissions(path);
        if std::env::var("USER").unwrap_or_default() == "root" {
            // If running as root, should pass
            assert!(result.is_ok());
        } else {
            // If running as non-root, should fail ownership check
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_check_config_permissions_dontblameme_bypass() {
        // Create a temporary file with insecure permissions (644)
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        
        // Set permissions to 644 (owner read/write, group read, others read)
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(path, perms).unwrap();
        
        // Set DONTBLAMEME=1 to bypass permission checks
        unsafe { std::env::set_var("DONTBLAMEME", "1"); }
        
        // Should pass due to bypass
        assert!(Config::check_config_permissions(path).is_ok());
        
        // Clean up environment variable
        unsafe { std::env::remove_var("DONTBLAMEME"); }
    }

    #[test]
    fn test_check_config_permissions_insecure() {
        // Create a temporary file with insecure permissions (644)
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        
        // Set permissions to 644 (owner read/write, group read, others read)
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(path, perms).unwrap();
        
        // Should fail due to insecure permissions
        // Note: If running as root, this will fail ownership check instead
        let result = Config::check_config_permissions(path);
        assert!(result.is_err(), "Expected permission check to fail");
    }

    #[test]
    fn test_config_validation_success() {
        let config = Config {
            ldap: LdapConfig {
                url: "ldaps://ldap.example.com:636".to_string(),
                bind_dn: "cn=admin,dc=example,dc=com".to_string(),
                bind_password: "secret".to_string(),
            },
            server: ServerConfig {
                bind_addr: "127.0.0.1:8080".parse().unwrap(),
                refresh_interval_secs: 180,
            },
            endpoints: vec![
                EndpointConfig {
                    path: "/groups".to_string(),
                    search_base: "ou=groups,dc=example,dc=com".to_string(),
                    search_filter: "(cn={})".to_string(),
                    search_scope: "subtree".to_string(),
                    attribute: "member".to_string(),
                    result_processing: Some(ResultProcessing {
                        r#type: "dn_translation".to_string(),
                        attribute: "uid".to_string(),
                    }),
                }
            ],
        };
        
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_endpoints() {
        let config = Config {
            ldap: LdapConfig {
                url: "ldaps://ldap.example.com:636".to_string(),
                bind_dn: "cn=admin,dc=example,dc=com".to_string(),
                bind_password: "secret".to_string(),
            },
            server: ServerConfig {
                bind_addr: "127.0.0.1:8080".parse().unwrap(),
                refresh_interval_secs: 180,
            },
            endpoints: vec![],
        };
        
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_endpoint_validation_invalid_path() {
        let endpoint = EndpointConfig {
            path: "groups".to_string(), // Missing leading slash
            search_base: "ou=groups,dc=example,dc=com".to_string(),
            search_filter: "(cn={})".to_string(),
            search_scope: "subtree".to_string(),
            attribute: "member".to_string(),
            result_processing: None,
        };
        
        assert!(endpoint.validate(0).is_err());
    }

    #[test]
    fn test_endpoint_validation_missing_placeholder() {
        let endpoint = EndpointConfig {
            path: "/groups".to_string(),
            search_base: "ou=groups,dc=example,dc=com".to_string(),
            search_filter: "(cn=groupname)".to_string(), // Missing {} placeholder
            search_scope: "subtree".to_string(),
            attribute: "member".to_string(),
            result_processing: None,
        };
        
        assert!(endpoint.validate(0).is_err());
    }
}

