mod config;
mod handler;
mod ldap;

use axum::{Router, routing::get};
use config::Config;
use handler::group_members;
use log::{debug, error, info};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::time::{Duration, interval};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub cache: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().filter_or("LOG_LEVEL", "info")).init();

    let config = Arc::new(Config::from_env()?);
    let cache = Arc::new(Mutex::new(HashMap::new()));
    let app_state = Arc::new(AppState {
        config: config.clone(),
        cache: cache.clone(),
    });

    // Spawn refresh task
    let config_for_task = config.clone();
    let cache_for_task = cache.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(
            *config_for_task.refresh_interval_secs(),
        ));

        loop {
            ticker.tick().await;

            let group_names: Vec<String> = {
                let guard = cache_for_task.lock().unwrap();
                guard.keys().cloned().collect()
            };

            if group_names.is_empty() {
                continue;
            }

            info!("Refreshing {} group(s)", group_names.len());

            if let Ok(mut ldap) = ldap::connect_and_bind(
                config_for_task.ldap_url(),
                config_for_task.bind_dn(),
                config_for_task.bind_password(),
            )
            .await
            {
                for name in group_names {
                    let group_dn = format!("cn={},{}", name, config_for_task.group_search_base());
                    if let Ok(member_dns) = ldap::get_group_member_dns(&mut ldap, &group_dn).await {
                        let uids = ldap::resolve_uids(&mut ldap, &member_dns).await;
                        let mut guard = cache_for_task.lock().unwrap();
                        guard.insert(name.clone(), uids);
                        debug!("Refreshed group: {}", name);
                    } else {
                        error!("Failed to refresh group: {}", name);
                    }
                }
            } else {
                error!("Failed to reconnect to LDAP for refresh");
            }
        }
    });

    info!("Server binding to {}", config.bind_addr());
    let listener = tokio::net::TcpListener::bind(config.bind_addr()).await?;

    let app = Router::new()
        .route("/group_members", get(group_members))
        .with_state(app_state);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
