# Documentation

Top-level index of cloud-manage-rs documentation.

## General

- [Getting Started](getting-started.md) — install, run, first instance.
- [Configuration Reference](configuration.md) — TOML schema for
  `./config/config` and the wizard's input model.
- [Web UI](web-ui.md) — `serve` subcommand, REST API, Snipe SSE stream.

## Providers

Provider-specific guides (auth setup, supported resources, quirks).

- [Oracle Cloud Infrastructure](providers/oracle.md)
- _AWS — planned_
- _Azure — planned_

## Internal

- The [OCI Rust SDK](../sdk/oci-rust-sdk) is vendored as a sub-crate; see its
  own README for SDK-level details.
