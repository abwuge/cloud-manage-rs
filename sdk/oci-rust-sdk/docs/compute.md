# Compute Client

## ComputeClient

Client for OCI Compute service operations.

### Constructor

#### `new(config: &dyn ConfigurationProvider) -> Result<Self>`

Create a new compute client.

**Parameters:**
- `config` - Configuration provider

**Returns:** `Result<ComputeClient, Box<dyn std::error::Error>>`

**Example:**
```rust
use oci_rust_sdk::compute::ComputeClient;

let client = ComputeClient::new(&config)?;
```

## Resource Queries

### `list_availability_domains(&self, compartment_id: &str) -> Result<Vec<AvailabilityDomain>>`

List all availability domains in a compartment.

**Parameters:**
- `compartment_id` - Compartment OCID

**Returns:** `Result<Vec<AvailabilityDomain>, Box<dyn std::error::Error>>`

**Example:**
```rust
let domains = client.list_availability_domains(&compartment_id).await?;
for domain in domains {
    println!("Domain: {}", domain.name);
}
```

### `list_images(&self, compartment_id: &str) -> Result<Vec<Image>>`

List all images available in a compartment.

**Note:** Without filters, this API returns a limited set of "featured" platform images (typically ~100 images, mostly Windows). To get complete image lists for specific operating systems, use `list_images_filtered()` instead.

**Parameters:**
- `compartment_id` - Compartment OCID

**Returns:** `Result<Vec<Image>, Box<dyn std::error::Error>>`

**Example:**
```rust
let images = client.list_images(&compartment_id).await?;
for image in images {
    println!("Image: {} - {}", 
        image.display_name.unwrap_or_default(),
        image.operating_system.unwrap_or_default()
    );
}
```

### `list_images_filtered(&self, compartment_id: &str, operating_system: Option<&str>, operating_system_version: Option<&str>) -> Result<Vec<Image>>`

List images with optional filters for operating system and version. This is the recommended way to get complete image lists.

**Parameters:**
- `compartment_id` - Compartment OCID
- `operating_system` - Optional OS filter (e.g., "Oracle Linux", "Canonical Ubuntu", "Windows")
- `operating_system_version` - Optional OS version filter (e.g., "8", "9", "22.04")

**Returns:** `Result<Vec<Image>, Box<dyn std::error::Error>>`

**Supported Operating Systems:**
- "Oracle Linux" - Oracle's Linux distribution
- "Canonical Ubuntu" - Ubuntu images
- "Windows" - Windows Server images
- "CentOS" - CentOS images
- "Red Hat Enterprise Linux" - RHEL (requires subscription)

**Example:**
```rust
// Get all Oracle Linux images
let ol_images = client.list_images_filtered(
    &compartment_id,
    Some("Oracle Linux"),
    None
).await?;

// Get Oracle Linux 9 images only
let ol9_images = client.list_images_filtered(
    &compartment_id,
    Some("Oracle Linux"),
    Some("9")
).await?;

// Get all Ubuntu images
let ubuntu_images = client.list_images_filtered(
    &compartment_id,
    Some("Canonical Ubuntu"),
    None
).await?;

for image in ol9_images {
    println!("Image: {} - {} {}", 
        image.display_name.unwrap_or_default(),
        image.operating_system.unwrap_or_default(),
        image.operating_system_version.unwrap_or_default()
    );
}
```

### `list_shapes(&self, compartment_id: &str) -> Result<Vec<Shape>>`

List all instance shapes available in a compartment.

**Parameters:**
- `compartment_id` - Compartment OCID

**Returns:** `Result<Vec<Shape>, Box<dyn std::error::Error>>`

**Example:**
```rust
let shapes = client.list_shapes(&compartment_id).await?;
for shape in shapes {
    println!("Shape: {} - {} OCPUs, {} GB memory",
        shape.shape,
        shape.ocpus.unwrap_or(0.0),
        shape.memory_in_gbs.unwrap_or(0.0)
    );
}
```

### `list_vcns(&self, compartment_id: &str) -> Result<Vec<Vcn>>`

List all Virtual Cloud Networks in a compartment.

**Parameters:**
- `compartment_id` - Compartment OCID

**Returns:** `Result<Vec<Vcn>, Box<dyn std::error::Error>>`

**Example:**
```rust
let vcns = client.list_vcns(&compartment_id).await?;
for vcn in vcns {
    println!("VCN: {} ({})",
        vcn.display_name.unwrap_or_default(),
        vcn.cidr_block.unwrap_or_default()
    );
}
```

### `list_subnets(&self, compartment_id: &str) -> Result<Vec<Subnet>>`

List all subnets in a compartment.

**Parameters:**
- `compartment_id` - Compartment OCID

**Returns:** `Result<Vec<Subnet>, Box<dyn std::error::Error>>`

**Example:**
```rust
let subnets = client.list_subnets(&compartment_id).await?;
for subnet in subnets {
    println!("Subnet: {} ({})",
        subnet.display_name.unwrap_or_default(),
        subnet.cidr_block.unwrap_or_default()
    );
}
```

## Instance Management

### `launch_instance(&self, details: &LaunchInstanceDetails) -> Result<Instance>`

Launch a new compute instance.

**Parameters:**
- `details` - Instance launch configuration

**Returns:** `Result<Instance, Box<dyn std::error::Error>>`

**Example:**
```rust
use oci_rust_sdk::compute::{
    LaunchInstanceDetails, InstanceSourceDetails, CreateVnicDetails
};

let launch_details = LaunchInstanceDetails {
    availability_domain: "Uocm:PHX-AD-1".to_string(),
    compartment_id: compartment_id.clone(),
    shape: "VM.Standard.E2.1.Micro".to_string(),
    display_name: Some("my-instance".to_string()),
    hostname_label: Some("my-instance".to_string()),
    source_details: InstanceSourceDetails::Image {
        image_id: "ocid1.image.oc1.phx.aaaaaa...".to_string(),
        boot_volume_size_in_gbs: None,
    },
    create_vnic_details: Some(CreateVnicDetails {
        subnet_id: "ocid1.subnet.oc1.phx.aaaaaa...".to_string(),
        assign_public_ip: Some(true),
        display_name: Some("my-vnic".to_string()),
        hostname_label: None,
        private_ip: None,
    }),
    metadata: None,
    shape_config: None,
    freeform_tags: None,
};

let instance = client.launch_instance(&launch_details).await?;
println!("Instance ID: {}", instance.id);
```

### `get_instance(&self, instance_id: &str) -> Result<Instance>`

Get details of a specific instance.

**Parameters:**
- `instance_id` - Instance OCID

**Returns:** `Result<Instance, Box<dyn std::error::Error>>`

**Example:**
```rust
let instance = client.get_instance("ocid1.instance.oc1.phx.aaaaaa...").await?;
println!("State: {:?}", instance.lifecycle_state);
```

### `terminate_instance(&self, instance_id: &str) -> Result<()>`

Terminate a compute instance.

**Parameters:**
- `instance_id` - Instance OCID

**Returns:** `Result<(), Box<dyn std::error::Error>>`

**Example:**
```rust
client.terminate_instance("ocid1.instance.oc1.phx.aaaaaa...").await?;
println!("Instance terminated");
```
