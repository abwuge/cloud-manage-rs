# Cloudflare Rust SDK

Small hand-crafted Cloudflare SDK used by cloud-manage-rs.

Cloudflare API access is intentionally limited to API token authentication.
The DNS client currently covers:

| Operation | Method |
| --- | --- |
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
    zone_id: String,
}

impl ConfigurationProvider for Config {
    fn api_token(&self) -> Result<String> {
        Self::require_value("api_token", &self.api_token)
    }

    fn zone_id(&self) -> Result<String> {
        Self::require_value("zone_id", &self.zone_id)
    }
}
```
