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
use oci_rust_sdk::compute::models::LifecycleState;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::ReceiverStream;

use crate::common::utils::build_instance_config;
use crate::config::InstanceConfigFile;
use crate::dns;
use crate::instance::{self, SnipeEvent};
use crate::providers::oracle::{
    AlwaysFreeInstanceType, InstanceNetworkSummary, OracleInstanceCreator, VnicIpv4Target,
};

use super::server::AppState;

const CONFIG_FILE: &str = "./config/config";

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
pub struct VnicView {
    pub vnic_id: String,
    pub display_name: Option<String>,
    pub is_primary: bool,
    pub nic_index: Option<i32>,
    pub subnet_id: Option<String>,
    pub mac_address: Option<String>,
    pub private_ip_id: String,
    pub private_ip: String,
    pub is_primary_private_ip: bool,
    pub public_ip_id: Option<String>,
    pub public_ip: Option<String>,
    pub public_ip_lifetime: Option<String>,
    pub public_ip_error: Option<String>,
    pub ipv6_addresses: Vec<String>,
    pub can_refresh: bool,
    pub refresh_disabled_reason: Option<String>,
}

impl VnicView {
    fn from_target(v: VnicIpv4Target, instance_state: &LifecycleState) -> Self {
        let disabled_reason = if *instance_state != LifecycleState::Running {
            Some(format!(
                "Instance is {}",
                format!("{:?}", instance_state).to_uppercase()
            ))
        } else if !v.is_primary_private_ip {
            Some("Only a VNIC's primary private IP supports ephemeral public IPv4".to_string())
        } else if v
            .public_ip_lifetime
            .as_deref()
            .is_some_and(|l| l.eq_ignore_ascii_case("RESERVED"))
        {
            Some("Reserved public IPv4 cannot be rotated automatically".to_string())
        } else if v.public_ip.is_some() && v.public_ip_id.is_none() {
            Some("Public IP OCID unavailable".to_string())
        } else {
            None
        };

        Self {
            vnic_id: v.vnic_id,
            display_name: v.vnic_display_name,
            is_primary: v.is_primary_vnic,
            nic_index: v.nic_index,
            subnet_id: v.subnet_id,
            mac_address: v.mac_address,
            private_ip_id: v.private_ip_id,
            private_ip: v.private_ip,
            is_primary_private_ip: v.is_primary_private_ip,
            public_ip_id: v.public_ip_id,
            public_ip: v.public_ip,
            public_ip_lifetime: v.public_ip_lifetime,
            public_ip_error: v.public_ip_error,
            ipv6_addresses: v.ipv6_addresses,
            can_refresh: disabled_reason.is_none(),
            refresh_disabled_reason: disabled_reason,
        }
    }
}

#[derive(Serialize)]
pub struct InstanceTargetView {
    pub instance_id: String,
    pub display_name: Option<String>,
    pub shape: String,
    pub ocpus: Option<f32>,
    pub memory_in_gbs: Option<f32>,
    pub availability_domain: String,
    pub region: Option<String>,
    pub lifecycle_state: String,
    pub compartment_id: String,
    pub time_created: Option<String>,
    pub primary_public_ip: Option<String>,
    pub vnics: Vec<VnicView>,
}

impl From<InstanceNetworkSummary> for InstanceTargetView {
    fn from(s: InstanceNetworkSummary) -> Self {
        let primary_public_ip = s.primary_vnic().and_then(|v| v.public_ip.clone());
        let state_str = format!("{:?}", s.lifecycle_state).to_uppercase();
        let state = s.lifecycle_state;
        let vnics = s
            .vnics
            .into_iter()
            .map(|v| VnicView::from_target(v, &state))
            .collect();
        Self {
            instance_id: s.instance_id,
            display_name: s.display_name,
            shape: s.shape,
            ocpus: s.ocpus,
            memory_in_gbs: s.memory_in_gbs,
            availability_domain: s.availability_domain,
            region: s.region,
            lifecycle_state: state_str,
            compartment_id: s.compartment_id,
            time_created: s.time_created,
            primary_public_ip,
            vnics,
        }
    }
}

fn build_config_view(cfg: &InstanceConfigFile) -> ConfigView {
    ConfigView {
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
    }
}

#[derive(Deserialize)]
pub struct UpdateConfigBody {
    pub instance: Option<UpdateInstanceBody>,
    pub network: Option<UpdateNetworkBody>,
    pub snipe: Option<UpdateSnipeBody>,
    pub cloudflare: Option<UpdateCloudflareBody>,
}

#[derive(Deserialize)]
pub struct UpdateInstanceBody {
    pub instance_type: String,
    pub display_name: String,
    pub arm_ocpus: Option<u8>,
    pub arm_memory_gb: Option<u8>,
    pub boot_volume_size_gb: i64,
}

#[derive(Deserialize)]
pub struct UpdateNetworkBody {
    pub assign_public_ip: bool,
    pub assign_ipv6: bool,
    pub private_ip: Option<String>,
    pub ipv6_address: Option<String>,
    pub hostname_label: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateSnipeBody {
    pub min_delay_secs: f64,
    pub max_delay_secs: f64,
    pub max_attempts: u32,
}

#[derive(Deserialize)]
pub struct UpdateCloudflareBody {
    pub zone_name: String,
    pub record_name: Option<String>,
    /// Leave omitted or empty to keep the existing token.
    pub api_token: Option<String>,
}

fn clean_opt(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

// ---------- Handlers ----------

pub async fn get_config(State(state): State<AppState>) -> ApiResult<Json<ConfigView>> {
    let cfg = state.config.read().await;
    Ok(Json(build_config_view(&cfg)))
}

pub async fn update_config(
    State(state): State<AppState>,
    Json(body): Json<UpdateConfigBody>,
) -> ApiResult<Json<ConfigView>> {
    let mut next = state.config.read().await.clone();

    if let Some(inst) = body.instance {
        let kind = inst.instance_type.trim().to_lowercase();
        if kind != "amd" && kind != "arm" {
            return Err(ApiError::bad_request("instance_type must be 'amd' or 'arm'"));
        }
        let name = inst.display_name.trim().to_string();
        if name.is_empty() {
            return Err(ApiError::bad_request("display_name is required"));
        }
        if inst.boot_volume_size_gb < 47 {
            return Err(ApiError::bad_request(
                "boot_volume_size_gb must be at least 47",
            ));
        }
        if kind == "arm" {
            let ocpus = inst.arm_ocpus.unwrap_or(2);
            let memory_gb = inst.arm_memory_gb.unwrap_or(ocpus.saturating_mul(6));
            AlwaysFreeInstanceType::ArmFlex { ocpus, memory_gb }
                .validate()
                .map_err(ApiError::bad_request)?;
            next.instance.arm_ocpus = Some(ocpus);
            next.instance.arm_memory_gb = Some(memory_gb);
        }
        next.instance.instance_type = kind;
        next.instance.display_name = name;
        next.instance.boot_volume_size_gb = inst.boot_volume_size_gb;
    }

    if let Some(net) = body.network {
        next.network.assign_public_ip = net.assign_public_ip;
        next.network.assign_ipv6 = net.assign_ipv6;
        next.network.private_ip = clean_opt(net.private_ip);
        next.network.ipv6_address = clean_opt(net.ipv6_address);
        next.network.hostname_label = clean_opt(net.hostname_label);
    }

    if let Some(snipe) = body.snipe {
        if !snipe.min_delay_secs.is_finite()
            || !snipe.max_delay_secs.is_finite()
            || snipe.min_delay_secs < 0.0
            || snipe.max_delay_secs < 0.0
        {
            return Err(ApiError::bad_request("snipe delays must be non-negative"));
        }
        next.snipe.min_delay_secs = snipe.min_delay_secs;
        next.snipe.max_delay_secs = snipe.max_delay_secs;
        next.snipe.max_attempts = snipe.max_attempts;
    }

    if let Some(cf) = body.cloudflare {
        let raw_zone = cf.zone_name.trim();
        if let Some((record_part, zone_part)) = raw_zone.split_once('@') {
            next.cloudflare.zone_name = zone_part.trim().to_string();
            next.cloudflare.record_name = clean_opt(Some(record_part.to_string()));
        } else {
            next.cloudflare.zone_name = raw_zone.to_string();
            next.cloudflare.record_name = clean_opt(cf.record_name);
        }
        if let Some(token) = clean_opt(cf.api_token) {
            next.cloudflare.api_token = token;
        }
    }

    next.save_to_file(CONFIG_FILE)?;
    let view = build_config_view(&next);
    *state.config.write().await = next;
    Ok(Json(view))
}

pub async fn list_instances(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<InstanceTargetView>>> {
    let config = state.config.read().await.clone();
    let summaries = instance::load_instance_networks(&config).await?;
    Ok(Json(summaries.into_iter().map(Into::into).collect()))
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
    #[serde(default)]
    pub vnic_id: Option<String>,
    #[serde(default)]
    pub private_ip_id: Option<String>,
}

#[derive(Serialize)]
pub struct RefreshIpResponse {
    pub vnic_id: String,
    pub private_ip_id: String,
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
    let body = body.map(|b| b.0).unwrap_or_default();
    let config = state.config.read().await.clone();

    let creator = OracleInstanceCreator::new(config.clone());
    let target = creator
        .public_ipv4_target_for_instance_id(
            &id,
            body.vnic_id.as_deref(),
            body.private_ip_id.as_deref(),
        )
        .await?;
    let result = creator.refresh_public_ipv4_target(&target).await?;

    let mut dns_updated: Vec<DnsRecordView> = Vec::new();
    if body.update_dns {
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
        vnic_id: result.vnic_id,
        private_ip_id: result.private_ip_id,
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

pub async fn update_dns(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpsertDnsBody>,
) -> ApiResult<Json<DnsRecordView>> {
    if body.name.trim().is_empty() || body.content.trim().is_empty() {
        return Err(ApiError::bad_request("name and content are required"));
    }
    let config = state.config.read().await.clone();
    let client = DnsClient::new(&config.cloudflare)?;
    let record = client
        .update_record(
            &id,
            &DnsRecordRequest {
                record_type: body.record_type,
                name: body.name,
                content: body.content,
                ttl: body.ttl,
                proxied: body.proxied,
            },
            body.proxied,
        )
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
    let (cancel_tx, cancel_rx) = watch::channel(false);

    let tx_watch = tx.clone();
    tokio::spawn(async move {
        tx_watch.closed().await;
        let _ = cancel_tx.send(true);
    });

    tokio::spawn(async move {
        let tx_cb = tx.clone();
        let _ = instance::snipe_instance_cancellable(
            &config,
            min_delay,
            max_delay,
            max_attempts,
            bypass,
            cancel_rx,
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
