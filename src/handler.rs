use std::sync::Arc;

use axum::{
    extract::{Path, State, Request},
    response::Json,
    routing::get,
    Router,
};
use log::{debug, info, warn};

use crate::{
    AppState,
    ldap::{connect_and_bind, query},
    config::{Config, EndpointConfig},
};

/// Shared function to execute LDAP queries and process results
/// Used by both the handler and the refresh logic
pub async fn execute_ldap_query(
    ldap: &mut ldap3::Ldap,
    endpoint: &EndpointConfig,
    name: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let filter_template = endpoint.search_filter();
    let filter = format!("{}", filter_template.replace("{}", name));
    
    debug!("Executing LDAP query: endpoint='{}', name='{}', filter='{}'", endpoint.path(), name, filter);
    debug!("LDAP search parameters: base='{}', scope='{}', attr='{}'", endpoint.search_base(), endpoint.search_scope(), endpoint.attribute());

    let values = query(ldap, endpoint.search_base(), endpoint.search_scope(), &filter, endpoint.attribute())
        .await?;

    let mut final_result = values.clone();
    debug!("Initial LDAP query returned {} values", values.len());

    // Apply result processing if configured
    if let Some(processing) = endpoint.result_processing() {
        match processing.r#type().as_str() {
            "dn_translation" => {
                debug!("Applying DN translation for {} values", values.len());
                let mut processed_values = vec![];
                for val in &values {
                    let res = query(ldap, val, "base", "(objectClass=*)", processing.attribute())
                        .await?;
                    processed_values.extend(res);
                }
                final_result = processed_values;
                debug!("DN translation completed: {} -> {} results", values.len(), final_result.len());
            }
            other => {
                warn!("Unknown processing type: {}", other);
            }
        }
    }

    debug!("Final result count: {} values", final_result.len());
    Ok(final_result)
}

pub async fn start_server(config: Arc<Config>, app_state: Arc<AppState>) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting web server on {}", config.server().bind_addr());
    
    let mut app = Router::new();
    
    // Dynamically create routes for all configured endpoints
    for endpoint in config.endpoints() {
        debug!("Adding route: {} -> generic_handler", endpoint.path());
        app = app.route(&format!("{}/:name", endpoint.path()), get(generic_handler));
    }
    
    let app = app.with_state(app_state);

    let listener = tokio::net::TcpListener::bind(config.server().bind_addr()).await?;
    info!("Server listening on {}", config.server().bind_addr());
    
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn generic_handler(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Json<Vec<String>> {
    let AppState { config, cache } = &*state;

    // Extract the endpoint path from the request
    let path = request.uri().path();
    // Split by '/' and get the first non-empty segment
    let endpoint_path = path.split('/')
        .filter(|s| !s.is_empty())
        .next()
        .unwrap_or("")
        .to_string();
    let full_endpoint_path = format!("/{}", endpoint_path);

    debug!("Received request: method='{}', path='{}', name='{}'", request.method(), full_endpoint_path, name);

    // Create a unique cache key that includes both endpoint and name
    let cache_key = format!("{}:{}", full_endpoint_path, name);

    // Check cache first
    {
        let cache_guard = cache.lock().unwrap();
        if let Some(cached) = cache_guard.get(&cache_key) {
            info!("Cache hit for '{}', returning {} cached results", cache_key, cached.len());
            debug!("Cache hit details: endpoint='{}', name='{}', result_count={}", full_endpoint_path, name, cached.len());
            return Json(cached.clone());
        }
    }

    debug!("Cache miss for '{}', querying LDAP", cache_key);

    // Find which endpoint this request is for by matching the request path
    let endpoint = config.endpoints()
        .iter()
        .find(|ep| *ep.path() == full_endpoint_path)
        .expect(&format!("No matching endpoint found for {}", full_endpoint_path));
    
    debug!("Using endpoint: {} with search_base: {}", endpoint.path(), endpoint.search_base());
    
    // If not in cache, query LDAP
    let mut ldap = connect_and_bind(config.ldap().url(), config.ldap().bind_dn(), config.ldap().bind_password())
        .await
        .expect("LDAP connect/bind failed");

    // Use the shared function to execute the LDAP query
    let final_result = execute_ldap_query(&mut ldap, endpoint, &name)
        .await
        .expect("Failed to execute LDAP query");

    // Cache the result
    {
        let mut cache_guard = cache.lock().unwrap();
        cache_guard.insert(cache_key.clone(), final_result.clone());
        debug!("Cache populated for '{}' with {} results", cache_key, final_result.len());
        
        // Log cache size for monitoring
        let cache_size = cache_guard.len();
        if cache_size > 100 {
            warn!("Cache size is large: {} entries", cache_size);
        } else {
            debug!("Cache size: {} entries", cache_size);
        }
    }

    info!("Request completed successfully: endpoint='{}', name='{}', result_count={}", full_endpoint_path, name, final_result.len());
    Json(final_result)
}
