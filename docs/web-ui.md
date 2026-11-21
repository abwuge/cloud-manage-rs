# Web UI

`cloud-manage` ships with an embedded web UI served by an in-process
[axum](https://docs.rs/axum) server. The static frontend under `web/` is
compiled into the binary via [`rust-embed`](https://docs.rs/rust-embed) with
no external CDN dependencies or Node build step.

## Launching

```bash
cloud-manage serve                              # http://127.0.0.1:7878
cloud-manage serve --port 9000                  # custom port
cloud-manage serve --host 0.0.0.0 --port 9000   # bind all interfaces
```

The server loads `./config/config` at startup. Updates saved from the
**Settings** panel persist to `./config/config` and apply immediately in
memory without restarting.

## Authentication

The web API supports a simple bearer-token scheme. When a token is
configured, **every** `/api/*` request must either carry an
`Authorization: Bearer <token>` header or append `?token=<token>` to the
URL (used for `EventSource` / SSE, which cannot set custom headers).

A token can be supplied two ways, in priority order:

1. `--token <value>` on the command line.
2. The `[web]` section of `./config/config`:

   ```toml
   [web]
   token = "replace-me-with-a-long-random-string"
   ```

If **no token is set**, the server starts without authentication. In that
case keep it bound to `127.0.0.1` or place it behind an auth-terminating
reverse proxy (nginx basic auth, Caddy `forward_auth`, Cloudflare Tunnel +
Access, etc.).

The frontend detects auth status via the public `GET /api/auth-status`
endpoint and, when auth is required, prompts the user for the token on
first load. The token is stored in `localStorage` under the key
`cloudManageToken`. A **Sign out** button in the top-right header clears
it.

> [!NOTE]
> The token is compared in constant time but is not hashed. Treat the
> config file (`./config/config`) as a secret — it already contains OCI
> private key paths and Cloudflare API tokens.

## Features

The web console exposes four panels:

| Panel | What it does |
| --- | --- |
| **Instances** | Lists Oracle Cloud instances with shape, lifecycle state, and all attached VNICs (primary and secondary), including public IPv4/IPv6, private IPv4, linked Cloudflare `A` records, per-VNIC public IPv4 rotation, and instance launch. |
| **DNS** | Lists, searches, filters, creates, updates, toggles proxy on, and deletes Cloudflare DNS records for the configured zone, highlighting `A` records linked to active Oracle VNICs. |
| **Snipe** | Runs the instance creation retry loop over Server-Sent Events with live attempt logs, elapsed timer, retry countdown, and immediate server-side cancellation when stopped. |
| **Settings** | Edits instance defaults, network defaults, snipe defaults, and Cloudflare DNS settings live (saving to `./config/config`), and displays copyable Oracle Cloud connection IDs. |

Sensitive fields (`api_token`, OCI `passphrase`) are never returned by
`/api/config`; only `api_token_set` is exposed.

## REST API

Base path: `/api`. Responses are JSON; errors return `{ "error": "<message>" }` with a `4xx`/`5xx` status.

| Method | Path | Description |
| --- | --- | --- |
| `GET` | `/config` | Redacted view of the active configuration. |
| `PUT` | `/config` | Update instance, network, snipe, and Cloudflare settings in memory and on disk. |
| `GET` | `/instances` | List active instances with all attached VNICs (`is_primary`, private/public IPv4, IPv6, lifetime, refresh eligibility). |
| `POST` | `/instances` | Launch one instance. Body: `{ "display_name": "optional-override" }`. |
| `POST` | `/instances/:id/refresh-ip` | Rotate an ephemeral public IPv4. Body: `{ "update_dns": true, "vnic_id": "...", "private_ip_id": "..." }`. Defaults to the instance's primary VNIC when `vnic_id` and `private_ip_id` are omitted. |
| `GET` | `/dns?type=A&name=foo` | List Cloudflare DNS records (filters optional). |
| `POST` | `/dns` | Upsert a DNS record. Body: `{ type, name, content, ttl?, proxied? }`. |
| `PUT` | `/dns/:id` | Update an existing DNS record by id. |
| `DELETE` | `/dns/:id` | Delete a DNS record by id. |
| `GET` | `/snipe/stream` | Server-Sent Events stream for snipe progress. Query params: `min_delay`, `max_delay`, `max_attempts`, `bypass`, `token`. |
| `GET` | `/auth-status` | Unauthenticated. Returns `{ "auth_required": bool }`. |

### Snipe SSE events

Each event has a named `event:` type and a JSON `data:` payload:

| Event | Payload |
| --- | --- |
| `started` | `{ min_delay, max_delay, max_attempts, bypass }` |
| `attempt_start` | `{ attempt }` |
| `attempt_error` | `{ attempt, message, retryable }` |
| `waiting` | `{ attempt, delay_secs }` |
| `success` | `{ attempt, instance_id }` |
| `stopped` | `{ reason }` |

The stream ends on `success` or `stopped`. Closing the `EventSource` from the
browser cancels the in-flight server-side retry loop.

## Example: scripted refresh + DNS sync

```bash
curl -X POST http://127.0.0.1:7878/api/instances/ocid1.instance.oc1..xxx/refresh-ip \
  -H 'Content-Type: application/json' \
  -d '{"update_dns": true, "vnic_id": "ocid1.vnic.oc1..yyy"}'
```

```json
{
  "vnic_id": "ocid1.vnic.oc1..yyy",
  "private_ip_id": "ocid1.privateip.oc1..zzz",
  "old_public_ip": "203.0.113.10",
  "new_public_ip": "203.0.113.42",
  "dns_updated": [
    { "id": "...", "name": "app.example.com", "content": "203.0.113.42",
      "record_type": "A", "ttl": 1, "proxied": false }
  ]
}
```

## Architecture notes

- The server reuses the same provider modules as the CLI
  (`OracleInstanceCreator`, `cloudflare_rust_sdk::dns::DnsClient`,
  `dns::update_a_records_pointing_to_ip`).
- `snipe_instance_cancellable` (in `src/instance/operations.rs`) accepts a
  `tokio::sync::watch::Receiver<bool>` cancellation signal and an
  `FnMut(SnipeEvent)` callback.
- Static assets live in `web/` (`index.html`, `styles.css`, `js/*.js`) and are
  embedded at compile time by `StaticAssets` in `src/web/assets.rs`.
