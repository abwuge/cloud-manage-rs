# Documentation

Top-level index of cloud-manage-rs documentation.

## General

- [Getting Started](getting-started.md) — install, run, first instance.
- [Configuration Reference](configuration.md) — TOML schema for
  `instance_config.toml` and the wizard's input model.

## Providers

Provider-specific guides (auth setup, supported resources, quirks).

- [Oracle Cloud Infrastructure](providers/oracle.md)
- _AWS — planned_
- _Azure — planned_

## Internal

- The [OCI Rust SDK](../sdk/oci-rust-sdk) is vendored as a sub-crate; see its
  own README for SDK-level details.
