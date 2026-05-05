# Cloudflare Rust SDK

Small hand-crafted Cloudflare SDK used by cloud-manage-rs.

Cloudflare API access is intentionally limited to API token authentication.
The client is configured with a zone name instead of a zone ID. It resolves
the zone ID with `GET /zones?name={zone_name}` before DNS record operations.

The DNS client currently covers:

| Operation | Method |
| --- | --- |
| Resolve zone ID | `DnsClient::resolve_zone_id` |
| List DNS records | `DnsClient::list_records` |
| Create DNS record | `DnsClient::create_record` |
| Patch DNS record | `DnsClient::update_record` |
| Upsert DNS record | `DnsClient::upsert_record` |
| Delete DNS record | `DnsClient::delete_record` |

## Auth

Implement `auth::ConfigurationProvider`:

```rust
use cloudflare_rust_sdk::auth::{ConfigurationProvider, Result};

struct Config {
    api_token: String,
    zone_name: String,
}

impl ConfigurationProvider for Config {
    fn api_token(&self) -> Result<String> {
        Self::require_value("api_token", &self.api_token)
    }

    fn zone_name(&self) -> Result<String> {
        Self::require_value("zone_name", &self.zone_name)
    }
}
```
