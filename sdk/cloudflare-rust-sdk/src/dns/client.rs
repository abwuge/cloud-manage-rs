use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;

use crate::auth::ConfigurationProvider;

use super::models::{DnsRecord, DnsRecordBody, DnsRecordRequest};

const API_BASE: &str = "https://api.cloudflare.com/client/v4";

pub struct DnsClient {
    http_client: Client,
    api_token: String,
    zone_id: String,
}

impl DnsClient {
    /// Create a new DNS client using Cloudflare API token auth.
    pub fn new(
        config: &dyn ConfigurationProvider,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            http_client: Client::new(),
            api_token: config.api_token()?,
            zone_id: config.zone_id()?,
        })
    }

    /// List DNS records, optionally filtered by type and name.
    pub async fn list_records(
        &self,
        record_type: Option<&str>,
        name: Option<&str>,
    ) -> Result<Vec<DnsRecord>, Box<dyn std::error::Error + Send + Sync>> {
        let mut request = self.http_client.get(self.zone_records_url());
        if let Some(record_type) = record_type {
            request = request.query(&[("type", record_type)]);
        }
        if let Some(name) = name {
            request = request.query(&[("name", name)]);
        }
        self.send(request).await
    }

    /// Create a record or patch the first record matching the same type and name.
    pub async fn upsert_record(
        &self,
        record: &DnsRecordRequest,
    ) -> Result<DnsRecord, Box<dyn std::error::Error + Send + Sync>> {
        let existing = self
            .list_records(Some(&record.record_type), Some(&record.name))
            .await?;
        match existing.first() {
            Some(current) => {
                let proxied = record.proxied.or(current.proxied);
                self.update_record(&current.id, record, proxied).await
            }
            None => self.create_record(record).await,
        }
    }

    /// Create a DNS record.
    pub async fn create_record(
        &self,
        record: &DnsRecordRequest,
    ) -> Result<DnsRecord, Box<dyn std::error::Error + Send + Sync>> {
        let body = DnsRecordBody {
            record_type: &record.record_type,
            name: &record.name,
            content: &record.content,
            ttl: record.ttl,
            proxied: record.proxied,
        };
        self.send(self.http_client.post(self.zone_records_url()).json(&body))
            .await
    }

    /// Patch a DNS record by ID.
    pub async fn update_record(
        &self,
        record_id: &str,
        record: &DnsRecordRequest,
        proxied: Option<bool>,
    ) -> Result<DnsRecord, Box<dyn std::error::Error + Send + Sync>> {
        let body = DnsRecordBody {
            record_type: &record.record_type,
            name: &record.name,
            content: &record.content,
            ttl: record.ttl,
            proxied,
        };
        self.send(
            self.http_client
                .request(Method::PATCH, self.record_url(record_id))
                .json(&body),
        )
        .await
    }

    /// Delete a DNS record by ID.
    pub async fn delete_record(
        &self,
        record_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _: serde_json::Value = self
            .send(self.http_client.delete(self.record_url(record_id)))
            .await?;
        Ok(())
    }

    fn zone_records_url(&self) -> String {
        format!("{API_BASE}/zones/{}/dns_records", self.zone_id)
    }

    fn record_url(&self, record_id: &str) -> String {
        format!("{}/{}", self.zone_records_url(), record_id)
    }

    async fn send<T: for<'de> Deserialize<'de>>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        let response = request.bearer_auth(&self.api_token).send().await?;
        let status = response.status();
        let body = response.text().await?;
        let parsed: CloudflareResponse<T> = serde_json::from_str(&body).map_err(|e| {
            format!("failed to parse Cloudflare response ({status}): {e}; body: {body}")
        })?;
        if status != StatusCode::OK || !parsed.success {
            return Err(format_cloudflare_error(status, &parsed.errors).into());
        }
        Ok(parsed.result)
    }
}

#[derive(Debug, Deserialize)]
struct CloudflareResponse<T> {
    success: bool,
    result: T,
    #[serde(default)]
    errors: Vec<CloudflareMessage>,
}

#[derive(Debug, Deserialize)]
struct CloudflareMessage {
    code: u32,
    message: String,
}

fn format_cloudflare_error(status: StatusCode, errors: &[CloudflareMessage]) -> String {
    if errors.is_empty() {
        return format!("Cloudflare API error {status}");
    }
    let details = errors
        .iter()
        .map(|e| format!("{} ({})", e.message, e.code))
        .collect::<Vec<_>>()
        .join("; ");
    format!("Cloudflare API error {status}: {details}")
}
