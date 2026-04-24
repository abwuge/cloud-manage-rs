# OCI Rust SDK

Oracle Cloud Infrastructure SDK for Rust.

## Features

- User Principal (API Key) authentication
- Compute instance management
- Resource queries (availability domains, images, shapes, VCNs, subnets)

## Quick Start

### Configuration

Create `~/.oci/config`:

```ini
[DEFAULT]
user=ocid1.user.oc1..aaaaaaaa...
fingerprint=aa:bb:cc:dd:ee:ff:00:11:22:33:44:55:66:77:88:99
key_file=~/.oci/oci_api_key.pem
tenancy=ocid1.tenancy.oc1..aaaaaaaa...
region=us-ashburn-1
```

### Basic Usage

```rust
use oci_rust_sdk::auth::FileConfigProvider;
use oci_rust_sdk::compute::ComputeClient;
use std::path::Path;

let config = FileConfigProvider::from_file(Path::new("~/.oci/config"), "DEFAULT")?;
let client = ComputeClient::new(&config)?;

// List availability domains
let domains = client.list_availability_domains(&compartment_id).await?;

// Launch instance
let instance = client.launch_instance(&launch_details).await?;
```

### Examples

```bash
# Test authentication
cargo run --example getting_started

# List resources and create instance
cargo run --example create_instance
```

## Documentation

- [API Documentation](docs/) - Detailed API reference
  - [Authentication](docs/authentication.md)
  - [Compute Client](docs/compute.md)
  - [Data Models](docs/models.md)
- [Examples](examples/) - Code examples

## License

MIT
