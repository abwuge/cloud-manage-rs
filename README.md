# Cloud Manage RS

> [!WARNING]
> Work in Progress

A unified, multi-provider cloud management tool written in Rust. The goal is
a single CLI that can manage cloud servers, domains and related resources
across providers. The first supported provider is Oracle Cloud
Infrastructure.

## Supported Providers

| Provider | Status | Docs |
| --- | --- | --- |
| Oracle Cloud Infrastructure | Implemented | [docs/providers/oracle.md](docs/providers/oracle.md) |
| Cloudflare DNS | Implemented | [docs/providers/cloudflare.md](docs/providers/cloudflare.md) |
| AWS | Planned | — |
| Azure | Planned | — |

## Interfaces

- **Interactive TUI** — `cargo run` drops into a menu driven by `dialoguer`.
- **Non-interactive CLI** — every menu action is also a `clap` subcommand
  for scripting / cron / automation.
- **Embedded Web UI** — `cloud-manage serve` launches a self-contained
  axum server whose frontend is compiled into the binary. Single file, no
  Node build step. See [docs/web-ui.md](docs/web-ui.md).

## Documentation

- [docs/](docs/) — full documentation index
- [Getting Started](docs/getting-started.md)
- [Configuration Reference](docs/configuration.md)
- [Web UI](docs/web-ui.md)
- [Oracle Cloud Provider](docs/providers/oracle.md)

## Components

- [OCI Rust SDK](sdk/oci-rust-sdk) — hand-crafted Oracle Cloud Infrastructure
  SDK used by this tool
- [Cloudflare Rust SDK](sdk/cloudflare-rust-sdk) — hand-crafted Cloudflare DNS
  SDK used by this tool

## Quick Start

```bash
cargo run                              # interactive menu (runs wizard if needed)
cargo run -- show-config               # non-interactive: print current config
cargo run -- create                    # non-interactive: launch one instance
cargo run -- refresh-ip                # choose an instance, then refresh public IPv4
cargo run -- snipe --min-delay 3 --max-delay 10
cargo run -- dns list                  # list Cloudflare DNS records
cargo run -- dns upsert --type A --name app.example.com --content 203.0.113.10
cargo run -- serve                     # embedded web UI on http://127.0.0.1:7878
```

The wizard walks through everything needed to create your first instance.
The main menu also offers a **Snipe Mode** that retries creation with
randomized backoff — useful when Always Free capacity is exhausted. The
same actions are available as CLI subcommands for scripting; see
[Getting Started](docs/getting-started.md#7-non-interactive-cli) for the
full list.

## Development

```bash
cargo check
cargo test
cargo build --release
```

> [!WARNING]
> Individual commits are **not guaranteed to build in isolation**. Commits
> are split by responsibility (e.g. SDK changes, main-project changes, and
> documentation are kept in separate commits) rather than to be
> bisect-friendly. A refactor that touches both the SDK and its callers
> may leave the tree temporarily un-buildable between two adjacent commits;
> the tip of `master` is what is expected to compile.

## License

MIT
