use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, Query, State},
    response::Json,
};
use log::{debug, info};
use serde::Deserialize;

use crate::{
    AppState,
    ldap::{connect_and_bind, get_group_member_dns, resolve_uids},
};

#[derive(Deserialize)]
pub struct GroupQuery {
    pub name: String,
}

pub async fn group_members(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(params): Query<GroupQuery>,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<String>> {
    let AppState { config, cache } = &*state;

    info!("Received request from {} for group '{}'", addr, params.name);

    {
        let cache_guard = cache.lock().unwrap();
        if let Some(cached) = cache_guard.get(&params.name) {
            debug!("Cache hit for {}", &params.name);
            return Json(cached.clone());
        }
    }

    let group_dn = format!("cn={},{}", params.name, config.group_search_base());

    let mut ldap = connect_and_bind(config.ldap_url(), config.bind_dn(), config.bind_password())
        .await
        .expect("LDAP connect/bind failed");

    let member_dns = get_group_member_dns(&mut ldap, &group_dn)
        .await
        .expect("Failed to get group members");

    let uids = resolve_uids(&mut ldap, &member_dns).await;

    {
        let mut cache_guard = cache.lock().unwrap();
        cache_guard.insert(params.name.clone(), uids.clone());
        debug!("Cache populated for {}", &params.name);
    }

    Json(uids)
}
