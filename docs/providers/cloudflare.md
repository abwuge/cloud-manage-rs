# Cloudflare DNS Provider

Cloudflare support is focused on DNS records. Authentication is intentionally
limited to API tokens; email/global API key auth is not implemented.

## Token Setup

Create a Cloudflare API token scoped to the target zone with:

| Scope | Permission |
| --- | --- |
| Zone | DNS Edit |

No Cloudflare global API key or account email is used. The interactive setup
asks for the token and a domain string. Use either the zone name by itself or
the OpenWrt-style `record@zone` form:

| Input | Meaning |
| --- | --- |
| `example.com` | Manage records in the `example.com` zone. |
| `app@example.com` | Manage the `example.com` zone and use `app` as the default record name in prompts. |

The saved config looks like this:

```toml
[cloudflare]
api_token = "cf_api_token_with_zone_dns_edit"
zone_name = "example.com"
record_name = "app" # optional
```

You do not need to enter a zone ID. The SDK resolves it online by calling
Cloudflare's v4 API with the configured zone name, then uses the returned ID
for DNS record operations.

The repository includes a small hand-crafted SDK at
[`sdk/cloudflare-rust-sdk`](../../sdk/cloudflare-rust-sdk). Cloudflare does
not currently publish an official Rust SDK for the general REST API/DNS
surface, so the application code talks to this local SDK instead of embedding
HTTP request details directly in the CLI layer.

## CLI

```bash
cargo run -- dns list
cargo run -- dns list --type A
cargo run -- dns list --type A --name app.example.com
cargo run -- dns upsert --type A --name app.example.com --content 203.0.113.10
cargo run -- dns upsert --type CNAME --name www.example.com --content app.example.com --ttl 300 --proxied true
cargo run -- dns delete 023e105f4ecef8ad9ca31a8372d0c353
```

`upsert` searches by `type` and `name`. If a matching record exists, it is
patched; otherwise a new record is created. `ttl = 1` means Cloudflare
automatic TTL.

## API Coverage

Implemented endpoints:

| Operation | Endpoint |
| --- | --- |
| Resolve zone | `GET /zones?name={zone_name}` |
| List | `GET /zones/{zone_id}/dns_records` |
| Create | `POST /zones/{zone_id}/dns_records` |
| Patch | `PATCH /zones/{zone_id}/dns_records/{dns_record_id}` |
| Delete | `DELETE /zones/{zone_id}/dns_records/{dns_record_id}` |
