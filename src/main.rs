mod config;
mod ldap;
mod handler;

use log::{error, info, debug, warn};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::time::{Duration, interval};

use crate::{
    ldap::connect_and_bind,
    handler::{start_server, execute_ldap_query},
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<config::Config>,
    pub cache: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

async fn refresh_cache(config: Arc<config::Config>, cache: Arc<Mutex<HashMap<String, Vec<String>>>>) {
    debug!("Starting cache refresh cycle");
    
    // Connect to LDAP once for all refreshes
    let mut ldap = match connect_and_bind(config.ldap().url(), config.ldap().bind_dn(), config.ldap().bind_password()).await {
        Ok(ldap) => {
            debug!("Successfully connected to LDAP for cache refresh");
            ldap
        },
        Err(e) => {
            error!("Failed to connect to LDAP for cache refresh: {}", e);
            return;
        }
    };

    let mut refresh_count = 0;
    let mut error_count = 0;

    // Get a copy of all cached keys to refresh
    let keys_to_refresh: Vec<String> = {
        let cache_guard = cache.lock().unwrap();
        cache_guard.keys().cloned().collect()
    };

    if keys_to_refresh.is_empty() {
        debug!("No cached entries to refresh, skipping cycle");
        return;
    }

    info!("Refreshing {} cached entries", keys_to_refresh.len());
    debug!("Cache keys to refresh: {:?}", keys_to_refresh);

    for cache_key in keys_to_refresh {
        // Parse the cache key to extract endpoint and name
        let parts: Vec<&str> = cache_key.split(':').collect();
        if parts.len() != 2 {
            error!("Invalid cache key format: {}", cache_key);
            continue;
        }
        
        let endpoint_path = parts[0];
        let name = parts[1];
        debug!("Processing cache refresh for endpoint: '{}', name: '{}'", endpoint_path, name);

        // Find the matching endpoint configuration
        let endpoint = match config.endpoints().iter().find(|ep| ep.path() == endpoint_path) {
            Some(ep) => ep,
            None => {
                error!("No endpoint found for path: {}", endpoint_path);
                continue;
            }
        };

        // Refresh this cached entry
        match refresh_cached_entry(&mut ldap, endpoint, name, &cache).await {
            Ok(_) => {
                refresh_count += 1;
                debug!("Successfully refreshed cache for '{}'", cache_key);
            },
            Err(e) => {
                error!("Failed to refresh cache for {}: {}", cache_key, e);
                error_count += 1;
            }
        }
    }

    if error_count > 0 {
        warn!("Cache refresh completed with errors: {} refreshed, {} errors", refresh_count, error_count);
    } else {
        info!("Cache refresh completed successfully: {} refreshed", refresh_count);
    }
}

async fn refresh_cached_entry(
    ldap: &mut ldap3::Ldap,
    endpoint: &crate::config::EndpointConfig,
    name: &str,
    cache: &Arc<Mutex<HashMap<String, Vec<String>>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Refreshing cached entry: endpoint='{}', name='{}'", endpoint.path(), name);
    
    // Use the shared function to execute the LDAP query
    let final_result = execute_ldap_query(ldap, endpoint, name).await?;
    debug!("LDAP query returned {} results for '{}'", final_result.len(), name);

    // Update the cache with fresh data
    let cache_key = format!("{}:{}", endpoint.path(), name);
    {
        let mut cache_guard = cache.lock().unwrap();
        cache_guard.insert(cache_key.clone(), final_result);
        debug!("Updated cache with fresh data for '{}'", cache_key);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().filter_or("LOG_LEVEL", "info"))
        .format(|buf, record| {
            use std::io::Write;
            writeln!(buf, "[{} {}] {}", record.level(), record.target(), record.args())
        })
        .init();

    info!("Starting LDAP cache daemon v{}", env!("CARGO_PKG_VERSION"));
    
    let config = Arc::new(config::Config::get_config()?);
    info!("Configuration loaded successfully");
    debug!("Server will bind to: {}", config.server().bind_addr());
    debug!("LDAP server: {}", config.ldap().url());
    debug!("Configured endpoints: {:?}", config.endpoints().iter().map(|e| e.path()).collect::<Vec<_>>());
    
    let cache = Arc::new(Mutex::new(HashMap::new()));
    let app_state = Arc::new(AppState {
        config: config.clone(),
        cache: cache.clone(),
    });

    // Start the background cache refresh thread
    let refresh_config = config.clone();
    let refresh_cache_arc = cache.clone();
    let refresh_interval = Duration::from_secs(*config.server().refresh_interval_secs());
    
    info!("Starting background cache refresh thread with interval: {} seconds", config.server().refresh_interval_secs());
    debug!("Cache refresh interval: {} seconds", refresh_interval.as_secs());
    
    tokio::spawn(async move {
        let mut interval = interval(refresh_interval);
        debug!("Waiting for first refresh interval before starting cache refresh cycles");
        // Wait for the first TTL interval before starting refresh cycle
        interval.tick().await;
        
        loop {
            interval.tick().await;
            refresh_cache(refresh_config.clone(), refresh_cache_arc.clone()).await;
        }
    });

    // Start the web server
    info!("Starting web server...");
    start_server(config, app_state).await?;

    Ok(())
}
