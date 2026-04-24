# OCI Rust SDK

## Features Implemented

✅ **User Principal Authentication** - API Key based authentication  
✅ **Compute Instance Management** - Create, get, and terminate instances

## Authentication

```rust
use oci_rust_sdk::auth::FileConfigProvider;

// Load configuration from default location (~/.oci/config)
let config = FileConfigProvider::new()?;

// Or load from specific profile
let config = FileConfigProvider::from_profile("PRODUCTION")?;
```

## Compute Instance Management

### Create Instance

```rust
use oci_rust_sdk::compute::{
    ComputeClient, LaunchInstanceDetails, InstanceSourceDetails, CreateVnicDetails,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let config = FileConfigProvider::new()?;
    let compute_client = ComputeClient::new(&config)?;
    
    // Prepare launch details
    let launch_details = LaunchInstanceDetails {
        availability_domain: "Uocm:PHX-AD-1".to_string(),
        compartment_id: "ocid1.compartment.oc1..aaaaaa...".to_string(),
        shape: "VM.Standard2.1".to_string(),
        source_details: InstanceSourceDetails::Image {
            image_id: "ocid1.image.oc1.phx.aaaaaa...".to_string(),
            boot_volume_size_in_gbs: Some(50),
        },
        create_vnic_details: Some(CreateVnicDetails {
            subnet_id: "ocid1.subnet.oc1.phx.aaaaaa...".to_string(),
            assign_public_ip: Some(true),
            display_name: Some("my-vnic".to_string()),
            hostname_label: None,
            private_ip: None,
        }),
        display_name: Some("my-instance".to_string()),
        hostname_label: None,
        metadata: None,
        shape_config: None,
        freeform_tags: None,
    };
    
    // Launch instance
    let instance = compute_client.launch_instance(&launch_details).await?;
    println!("Instance created: {}", instance.id);
    println!("State: {:?}", instance.lifecycle_state);
    
    Ok(())
}
```

### Get Instance

```rust
let instance = compute_client.get_instance("ocid1.instance.oc1.phx.aaaaaa...").await?;
println!("Instance: {} - {:?}", instance.display_name.unwrap_or_default(), instance.lifecycle_state);
```

### Terminate Instance

```rust
compute_client.terminate_instance("ocid1.instance.oc1.phx.aaaaaa...").await?;
println!("Instance terminated");
```

## Configuration File Format

`~/.oci/config`:
```ini
[DEFAULT]
user=ocid1.user.oc1..aaaaaaaa...
fingerprint=aa:bb:cc:dd:ee:ff:00:11:22:33:44:55:66:77:88:99
key_file=~/.oci/oci_api_key.pem
tenancy=ocid1.tenancy.oc1..aaaaaaaa...
region=us-ashburn-1
```

## Module Structure

```
src/
├── auth/           - Authentication module
│   ├── config.rs   - ConfigurationProvider trait
│   ├── error.rs    - Error types
│   ├── file_config.rs - File-based config provider
│   └── signer.rs   - HTTP request signer
└── compute/        - Compute service module
    ├── client.rs   - ComputeClient implementation
    └── models.rs   - Data structures
```

## License

MIT
