use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post},
};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::config::InstanceConfigFile;

use super::api;
use super::assets::static_handler;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<InstanceConfigFile>>,
}

pub async fn serve(
    config: InstanceConfigFile,
    host: &str,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing only if not already set
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let state = AppState {
        config: Arc::new(RwLock::new(config)),
    };

    let api_router = Router::new()
        .route("/config", get(api::get_config))
        .route("/instances", get(api::list_instances))
        .route("/instances", post(api::create_instance))
        .route(
            "/instances/:id/refresh-ip",
            post(api::refresh_instance_ip),
        )
        .route("/dns", get(api::list_dns).post(api::upsert_dns))
        .route("/dns/:id", delete(api::delete_dns))
        .route("/snipe/stream", get(api::snipe_stream));

    let app = Router::new()
        .nest("/api", api_router)
        .fallback(static_handler)
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("\n🌐 Web UI listening on http://{}", addr);
    println!("   Press Ctrl+C to stop.\n");
    axum::serve(listener, app).await?;
    Ok(())
}
