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
| AWS | Planned | — |
| Azure | Planned | — |

## Documentation

- [docs/](docs/) — full documentation index
- [Getting Started](docs/getting-started.md)
- [Configuration Reference](docs/configuration.md)
- [Oracle Cloud Provider](docs/providers/oracle.md)

## Components

- [OCI Rust SDK](sdk/oci-rust-sdk) — hand-crafted Oracle Cloud Infrastructure
  SDK used by this tool

## Quick Start

```bash
cargo run
```

The wizard walks through everything needed to create your first instance.
The main menu also offers a **Snipe Mode** that retries creation with
randomized backoff — useful when Always Free capacity is exhausted.
See [Getting Started](docs/getting-started.md) for details.

## Development

```bash
cargo check
cargo test
cargo build --release
```

## License

MIT
