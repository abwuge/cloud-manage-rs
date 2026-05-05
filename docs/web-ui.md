# Web UI

`cloud-manage` ships with an embedded web UI served by an in-process
[axum](https://docs.rs/axum) server. The frontend (`web/index.html`) is
compiled into the binary via [`rust-embed`](https://docs.rs/rust-embed), so
the release build is a **single self-contained executable** — no Node tool
chain, no static file hosting, no extra deployment artifacts.

## Launching

```bash
cloud-manage serve                         # http://127.0.0.1:7878
cloud-manage serve --port 9000             # custom port
cloud-manage serve --host 0.0.0.0 --port 9000   # bind all interfaces
```

The server reads `./config/config` at startup (the same file used by every
other subcommand). Reconfiguring while the server is running requires a
restart.

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

The single-page app exposes four panels:

| Panel | What it does |
| --- | --- |
| **Overview** | Counters (instance total / running / DNS records / snipe window) and a read-only configuration summary. |
| **Instances** | Lists Oracle Cloud instances with lifecycle state & public IPv4. Launch a new instance from the current configuration; refresh a running instance's public IPv4 and optionally sync any Cloudflare `A` records that were pointing to the old IP. |
| **DNS** | Lists Cloudflare DNS records for the configured zone. Create / edit / delete records. |
| **Snipe** | Starts the same retry loop as `cloud-manage snipe`, streaming attempt-by-attempt progress into the browser over Server-Sent Events. Parameters (min/max delay, attempt cap, bypass) can be overridden per run without touching the config file. |

Sensitive fields (`api_token`, OCI `passphrase`) are never returned by
`/api/config`; only a boolean `api_token_set` is exposed so the UI can hint
that Cloudflare credentials are present.

## REST API

Base path: `/api`. All responses are JSON; errors use the shape
`{ "error": "<message>" }` with a `4xx`/`5xx` status code.

| Method | Path | Description |
| --- | --- | --- |
| `GET` | `/config` | Redacted view of the active configuration. |
| `GET` | `/instances` | List instances with lifecycle state and public IPv4. |
| `POST` | `/instances` | Launch one instance. Body: `{ "display_name": "optional-override" }`. |
| `POST` | `/instances/:id/refresh-ip` | Release & reallocate the ephemeral public IPv4. Body: `{ "update_dns": true }` to also rewrite matching Cloudflare `A` records. |
| `GET` | `/dns?type=A&name=foo` | List Cloudflare DNS records (filters optional). |
| `POST` | `/dns` | Upsert. Body: `{ type, name, content, ttl?, proxied? }`. |
| `DELETE` | `/dns/:id` | Delete a DNS record by id. |
| `GET` | `/snipe/stream` | Server-Sent Events stream for snipe progress. Query params: `min_delay`, `max_delay`, `max_attempts`, `bypass`, `token`. |
| `GET` | `/auth-status` | Unauthenticated. Returns `{ "auth_required": bool }` so the frontend can decide whether to prompt for a token. |

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

The stream terminates after `success` or `stopped`. Closing the
`EventSource` on the client does **not** abort an in-flight retry loop — the
loop will continue until it reaches its natural terminal state (success,
non-retryable error, or attempt cap). This mirrors the CLI `snipe`
behavior.

## Example: scripted refresh + DNS sync

```bash
curl -X POST http://127.0.0.1:7878/api/instances/ocid1.instance.oc1..xxx/refresh-ip \
  -H 'Content-Type: application/json' \
  -d '{"update_dns": true}'
```

```json
{
  "old_public_ip": "203.0.113.10",
  "new_public_ip": "203.0.113.42",
  "dns_updated": [
    { "id": "...", "name": "app.example.com", "content": "203.0.113.42",
      "record_type": "A", "ttl": 1, "proxied": false }
  ]
}
```

## Architecture notes

- The server reuses the exact same provider modules as the CLI
  (`OracleInstanceCreator`, `cloudflare_rust_sdk::dns::DnsClient`,
  `dns::update_a_records_pointing_to_ip`). There is no duplicated business
  logic between CLI and web.
- `snipe_instance_core` (in `src/instance/operations.rs`) takes an
  `FnMut(SnipeEvent)` callback. The CLI wraps it with a `println!`-based
  reporter; the web layer wraps it with an mpsc channel that is adapted to
  an SSE stream.
- Static assets live in `web/` and are embedded at compile time by the
  `StaticAssets` struct in `src/web/assets.rs`. Any unknown path falls back
  to `index.html` so that SPA-style routing works.
