use std::convert::Infallible;
use std::time::Duration;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response, Sse, sse::Event},
};
use cloudflare_rust_sdk::dns::{DnsClient, DnsRecord, DnsRecordRequest};
use futures::StreamExt;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::common::utils::build_instance_config;
use crate::dns;
use crate::instance::{self, SnipeEvent};
use crate::providers::oracle::{OracleInstanceCreator, PublicIpv4Target};

use super::server::AppState;

// ---------- Error helpers ----------

pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }
    fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: msg.into(),
        }
    }
}

impl<E: std::fmt::Display> From<E> for ApiError {
    fn from(err: E) -> Self {
        Self::internal(err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({ "error": self.message }));
        (self.status, body).into_response()
    }
}

type ApiResult<T> = Result<T, ApiError>;

// ---------- DTOs ----------

#[derive(Serialize)]
pub struct ConfigView {
    pub oracle: OracleView,
    pub instance: InstanceView,
    pub network: NetworkView,
    pub snipe: SnipeView,
    pub cloudflare: CloudflareView,
}

#[derive(Serialize)]
pub struct OracleView {
    pub region: String,
    pub tenancy: String,
    pub user: String,
    pub compartment_id: String,
    pub availability_domain: String,
    pub subnet_id: String,
    pub image_id_amd: String,
    pub image_id_arm: String,
}

#[derive(Serialize)]
pub struct InstanceView {
    pub instance_type: String,
    pub display_name: String,
    pub arm_ocpus: Option<u8>,
    pub arm_memory_gb: Option<u8>,
    pub boot_volume_size_gb: i64,
}

#[derive(Serialize)]
pub struct NetworkView {
    pub assign_public_ip: bool,
    pub assign_ipv6: bool,
    pub private_ip: Option<String>,
    pub ipv6_address: Option<String>,
    pub hostname_label: Option<String>,
}

#[derive(Serialize)]
pub struct SnipeView {
    pub min_delay_secs: f64,
    pub max_delay_secs: f64,
    pub max_attempts: u32,
}

#[derive(Serialize)]
pub struct CloudflareView {
    pub zone_name: String,
    pub record_name: Option<String>,
    pub api_token_set: bool,
}

#[derive(Serialize)]
pub struct InstanceTargetView {
    pub instance_id: String,
    pub display_name: Option<String>,
    pub lifecycle_state: String,
    pub public_ip: Option<String>,
    pub public_ip_error: Option<String>,
    pub vnic_display_name: Option<String>,
    pub compartment_id: String,
}

impl From<PublicIpv4Target> for InstanceTargetView {
    fn from(t: PublicIpv4Target) -> Self {
        Self {
            instance_id: t.instance_id,
            display_name: t.display_name,
            lifecycle_state: format!("{:?}", t.lifecycle_state).to_uppercase(),
            public_ip: t.public_ip,
            public_ip_error: t.public_ip_error,
            vnic_display_name: t.vnic_display_name,
            compartment_id: t.compartment_id,
        }
    }
}

// ---------- Handlers ----------

pub async fn get_config(State(state): State<AppState>) -> ApiResult<Json<ConfigView>> {
    let cfg = state.config.read().await;
    let view = ConfigView {
        oracle: OracleView {
            region: cfg.oci.region.clone(),
            tenancy: cfg.oci.tenancy.clone(),
            user: cfg.oci.user.clone(),
            compartment_id: cfg.oracle.compartment_id.clone(),
            availability_domain: cfg.oracle.availability_domain.clone(),
            subnet_id: cfg.oracle.subnet_id.clone(),
            image_id_amd: cfg.oracle.image_id_amd.clone(),
            image_id_arm: cfg.oracle.image_id_arm.clone(),
        },
        instance: InstanceView {
            instance_type: cfg.instance.instance_type.clone(),
            display_name: cfg.instance.display_name.clone(),
            arm_ocpus: cfg.instance.arm_ocpus,
            arm_memory_gb: cfg.instance.arm_memory_gb,
            boot_volume_size_gb: cfg.instance.boot_volume_size_gb,
        },
        network: NetworkView {
            assign_public_ip: cfg.network.assign_public_ip,
            assign_ipv6: cfg.network.assign_ipv6,
            private_ip: cfg.network.private_ip.clone(),
            ipv6_address: cfg.network.ipv6_address.clone(),
            hostname_label: cfg.network.hostname_label.clone(),
        },
        snipe: SnipeView {
            min_delay_secs: cfg.snipe.min_delay_secs,
            max_delay_secs: cfg.snipe.max_delay_secs,
            max_attempts: cfg.snipe.max_attempts,
        },
        cloudflare: CloudflareView {
            zone_name: cfg.cloudflare.zone_name.clone(),
            record_name: cfg.cloudflare.record_name.clone(),
            api_token_set: !cfg.cloudflare.api_token.trim().is_empty(),
        },
    };
    Ok(Json(view))
}

pub async fn list_instances(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<InstanceTargetView>>> {
    let config = state.config.read().await.clone();
    let targets = instance::load_public_ipv4_targets(&config).await?;
    Ok(Json(targets.into_iter().map(Into::into).collect()))
}

#[derive(Deserialize)]
pub struct CreateInstanceBody {
    /// Optional override for display name
    pub display_name: Option<String>,
}

#[derive(Serialize)]
pub struct CreateInstanceResponse {
    pub instance_id: String,
}

pub async fn create_instance(
    State(state): State<AppState>,
    Json(body): Json<CreateInstanceBody>,
) -> ApiResult<Json<CreateInstanceResponse>> {
    let config = state.config.read().await.clone();
    let mut instance_config = build_instance_config(&config);
    if let Some(name) = body.display_name {
        if !name.trim().is_empty() {
            instance_config.display_name = name;
        }
    }

    let creator = OracleInstanceCreator::new(config);
    let id = creator.create_instance(&instance_config).await?;
    Ok(Json(CreateInstanceResponse { instance_id: id }))
}

#[derive(Deserialize, Default)]
pub struct RefreshIpBody {
    #[serde(default)]
    pub update_dns: bool,
}

#[derive(Serialize)]
pub struct RefreshIpResponse {
    pub old_public_ip: Option<String>,
    pub new_public_ip: String,
    pub dns_updated: Vec<DnsRecordView>,
}

#[derive(Serialize)]
pub struct DnsRecordView {
    pub id: String,
    pub name: String,
    pub content: String,
    pub record_type: String,
    pub ttl: u32,
    pub proxied: Option<bool>,
}

impl From<DnsRecord> for DnsRecordView {
    fn from(r: DnsRecord) -> Self {
        Self {
            id: r.id,
            name: r.name,
            content: r.content,
            record_type: r.record_type,
            ttl: r.ttl,
            proxied: r.proxied,
        }
    }
}

pub async fn refresh_instance_ip(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<RefreshIpBody>>,
) -> ApiResult<Json<RefreshIpResponse>> {
    let update_dns = body.map(|b| b.0.update_dns).unwrap_or(false);
    let config = state.config.read().await.clone();

    let creator = OracleInstanceCreator::new(config.clone());
    let target = creator.public_ipv4_target_for_instance_id(&id).await?;
    let result = creator.refresh_public_ipv4_target(&target).await?;

    let mut dns_updated: Vec<DnsRecordView> = Vec::new();
    if update_dns {
        if let Some(old_ip) = result.old_public_ip.as_deref() {
            let recs = dns::update_a_records_pointing_to_ip(
                &config,
                old_ip,
                &result.new_public_ip,
            )
            .await?;
            dns_updated = recs.into_iter().map(Into::into).collect();
        }
    }

    Ok(Json(RefreshIpResponse {
        old_public_ip: result.old_public_ip,
        new_public_ip: result.new_public_ip,
        dns_updated,
    }))
}

#[derive(Deserialize)]
pub struct DnsListQuery {
    #[serde(rename = "type")]
    pub record_type: Option<String>,
    pub name: Option<String>,
}

pub async fn list_dns(
    State(state): State<AppState>,
    Query(q): Query<DnsListQuery>,
) -> ApiResult<Json<Vec<DnsRecordView>>> {
    let config = state.config.read().await.clone();
    let client = DnsClient::new(&config.cloudflare)?;
    let records = client
        .list_records(q.record_type.as_deref(), q.name.as_deref())
        .await?;
    Ok(Json(records.into_iter().map(Into::into).collect()))
}

#[derive(Deserialize)]
pub struct UpsertDnsBody {
    #[serde(rename = "type")]
    pub record_type: String,
    pub name: String,
    pub content: String,
    #[serde(default = "default_ttl")]
    pub ttl: u32,
    #[serde(default)]
    pub proxied: Option<bool>,
}

fn default_ttl() -> u32 {
    1
}

pub async fn upsert_dns(
    State(state): State<AppState>,
    Json(body): Json<UpsertDnsBody>,
) -> ApiResult<Json<DnsRecordView>> {
    if body.name.trim().is_empty() || body.content.trim().is_empty() {
        return Err(ApiError::bad_request("name and content are required"));
    }
    let config = state.config.read().await.clone();
    let client = DnsClient::new(&config.cloudflare)?;
    let record = client
        .upsert_record(&DnsRecordRequest {
            record_type: body.record_type,
            name: body.name,
            content: body.content,
            ttl: body.ttl,
            proxied: body.proxied,
        })
        .await?;
    Ok(Json(record.into()))
}

pub async fn delete_dns(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let config = state.config.read().await.clone();
    let client = DnsClient::new(&config.cloudflare)?;
    client.delete_record(&id).await?;
    Ok(Json(serde_json::json!({ "deleted": id })))
}

// ---------- Snipe SSE ----------

#[derive(Deserialize)]
pub struct SnipeQuery {
    #[serde(default)]
    pub min_delay: Option<f64>,
    #[serde(default)]
    pub max_delay: Option<f64>,
    #[serde(default)]
    pub max_attempts: Option<u32>,
    #[serde(default)]
    pub bypass: Option<bool>,
}

pub async fn snipe_stream(
    State(state): State<AppState>,
    Query(q): Query<SnipeQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let config = state.config.read().await.clone();
    let min_delay = q.min_delay.unwrap_or(config.snipe.min_delay_secs);
    let max_delay = q.max_delay.unwrap_or(config.snipe.max_delay_secs);
    let max_attempts = q.max_attempts.unwrap_or(config.snipe.max_attempts);
    let bypass = q.bypass.unwrap_or(false);

    let (tx, rx) = mpsc::channel::<SnipeEvent>(64);

    tokio::spawn(async move {
        let tx_cb = tx.clone();
        let _ = instance::snipe_instance_core(
            &config,
            min_delay,
            max_delay,
            max_attempts,
            bypass,
            move |event| {
                let _ = tx_cb.try_send(event);
            },
        )
        .await;
    });

    let stream = ReceiverStream::new(rx).map(|event| {
        let (kind, payload) = match &event {
            SnipeEvent::Started {
                min_delay,
                max_delay,
                max_attempts,
                bypass,
            } => (
                "started",
                serde_json::json!({
                    "min_delay": min_delay,
                    "max_delay": max_delay,
                    "max_attempts": max_attempts,
                    "bypass": bypass,
                }),
            ),
            SnipeEvent::AttemptStart { attempt } => (
                "attempt_start",
                serde_json::json!({ "attempt": attempt }),
            ),
            SnipeEvent::AttemptError {
                attempt,
                message,
                retryable,
            } => (
                "attempt_error",
                serde_json::json!({
                    "attempt": attempt,
                    "message": message,
                    "retryable": retryable,
                }),
            ),
            SnipeEvent::Waiting {
                attempt,
                delay_secs,
            } => (
                "waiting",
                serde_json::json!({ "attempt": attempt, "delay_secs": delay_secs }),
            ),
            SnipeEvent::Success {
                attempt,
                instance_id,
            } => (
                "success",
                serde_json::json!({ "attempt": attempt, "instance_id": instance_id }),
            ),
            SnipeEvent::Stopped { reason } => (
                "stopped",
                serde_json::json!({ "reason": reason }),
            ),
        };
        Ok(Event::default().event(kind).data(payload.to_string()))
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}
