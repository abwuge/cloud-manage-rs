use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use serde::Deserialize;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::config::InstanceConfigFile;

use super::api;
use super::assets::static_handler;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<InstanceConfigFile>>,
    pub auth_token: Option<Arc<String>>,
}

#[derive(Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

async fn auth_middleware(
    State(state): State<AppState>,
    Query(q): Query<TokenQuery>,
    req: Request,
    next: Next,
) -> Response {
    let Some(expected) = state.auth_token.as_deref() else {
        return next.run(req).await;
    };

    let header_token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    let provided = header_token.or(q.token);

    match provided {
        Some(t) if constant_time_eq(t.as_bytes(), expected.as_bytes()) => next.run(req).await,
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "unauthorized", "auth_required": true })),
        )
            .into_response(),
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Public endpoint used by the frontend to detect whether auth is required
/// (without leaking the actual token).
async fn auth_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "auth_required": state.auth_token.is_some(),
    }))
}

pub async fn serve(
    config: InstanceConfigFile,
    host: &str,
    port: u16,
    auth_token: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let auth_configured = auth_token.is_some();
    let state = AppState {
        config: Arc::new(RwLock::new(config)),
        auth_token: auth_token.map(Arc::new),
    };

    let protected = Router::new()
        .route("/config", get(api::get_config))
        .route("/instances", get(api::list_instances))
        .route("/instances", post(api::create_instance))
        .route(
            "/instances/:id/refresh-ip",
            post(api::refresh_instance_ip),
        )
        .route("/dns", get(api::list_dns).post(api::upsert_dns))
        .route("/dns/:id", delete(api::delete_dns))
        .route("/snipe/stream", get(api::snipe_stream))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // /api/auth-status is intentionally unauthenticated so the frontend can
    // detect whether to prompt for a token.
    let api_router = Router::new()
        .route("/auth-status", get(auth_status))
        .merge(protected);

    let app = Router::new()
        .nest("/api", api_router)
        .fallback(static_handler)
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("\n🌐 Web UI listening on http://{}", addr);
    if auth_configured {
        println!("   🔒 Bearer token auth: ENABLED");
    } else {
        println!("   ⚠️  No auth token configured — keep bound to 127.0.0.1");
    }
    println!("   Press Ctrl+C to stop.\n");
    axum::serve(listener, app).await?;
    Ok(())
}
