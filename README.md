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
cargo run                              # interactive menu (runs wizard if needed)
cargo run -- show-config               # non-interactive: print current config
cargo run -- create                    # non-interactive: launch one instance
cargo run -- snipe --min-delay 3 --max-delay 10
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
