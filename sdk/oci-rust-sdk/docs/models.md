# Data Models

## Resource Models

### AvailabilityDomain

Represents an availability domain.

**Fields:**
- `name: String` - Availability domain name
- `compartment_id: String` - Compartment OCID

### Image

Represents a compute image.

**Fields:**
- `id: String` - Image OCID
- `compartment_id: Option<String>` - Compartment OCID
- `display_name: Option<String>` - Display name
- `operating_system: Option<String>` - OS name
- `operating_system_version: Option<String>` - OS version
- `lifecycle_state: Option<String>` - Current state
- `time_created: Option<String>` - Creation timestamp

### Shape

Represents an instance shape (size/type).

**Fields:**
- `shape: String` - Shape name
- `processor_description: Option<String>` - Processor info
- `ocpus: Option<f32>` - Number of OCPUs
- `memory_in_gbs: Option<f32>` - Memory in GB
- `networking_bandwidth_in_gbps: Option<f32>` - Network bandwidth

### Vcn

Represents a Virtual Cloud Network.

**Fields:**
- `id: String` - VCN OCID
- `compartment_id: String` - Compartment OCID
- `display_name: Option<String>` - Display name
- `cidr_block: Option<String>` - CIDR block
- `lifecycle_state: String` - Current state

### Subnet

Represents a subnet within a VCN.

**Fields:**
- `id: String` - Subnet OCID
- `compartment_id: String` - Compartment OCID
- `vcn_id: String` - Parent VCN OCID
- `display_name: Option<String>` - Display name
- `cidr_block: Option<String>` - IPv4 CIDR block
- `ipv6_cidr_block: Option<String>` - IPv6 CIDR block (only set if the subnet has IPv6 enabled)
- `lifecycle_state: String` - Current state
- `availability_domain: Option<String>` - Availability domain

## Instance Models

### Instance

Represents a compute instance.

**Fields:**
- `id: String` - Instance OCID
- `compartment_id: String` - Compartment OCID
- `availability_domain: String` - Availability domain
- `lifecycle_state: LifecycleState` - Current state
- `shape: String` - Instance shape
- `region: String` - Region identifier
- `display_name: Option<String>` - Display name
- `time_created: Option<String>` - Creation timestamp
- `image_id: Option<String>` - Source image OCID (when launched from an image)
- `freeform_tags: Option<HashMap<String, String>>` - Free-form tags applied to the instance

### LaunchInstanceDetails

Configuration for launching a new instance.

**Fields:**
- `availability_domain: String` - Target availability domain
- `compartment_id: String` - Target compartment
- `shape: String` - Instance shape
- `display_name: Option<String>` - Display name
- `hostname_label: Option<String>` - Hostname
- `source_details: InstanceSourceDetails` - Boot source configuration
- `create_vnic_details: Option<CreateVnicDetails>` - Network configuration
- `metadata: Option<HashMap<String, String>>` - Custom metadata
- `shape_config: Option<LaunchInstanceShapeConfigDetails>` - Shape configuration
- `freeform_tags: Option<HashMap<String, String>>` - Tags

### InstanceSourceDetails

Boot source configuration (enum). Serialized with the `sourceType` discriminator.

**Variants:**
- `Image { image_id: String, boot_volume_size_in_gbs: Option<i64> }` - Boot from image
- `BootVolume { boot_volume_id: String }` - Boot from existing volume

### CreateVnicDetails

Network interface configuration.

**Fields:**
- `subnet_id: String` - Target subnet OCID
- `assign_public_ip: Option<bool>` - Assign a public IPv4
- `display_name: Option<String>` - Display name
- `hostname_label: Option<String>` - Hostname label
- `private_ip: Option<String>` - Specific private IPv4 in the subnet's CIDR
- `assign_ipv6_ip: Option<bool>` - Enable IPv6 on the VNIC. When `true` and `ipv6_address_ipv6_subnet_cidr_pair_details` is `None`, OCI auto-assigns an IPv6 address from the subnet's IPv6 CIDR.
- `ipv6_address_ipv6_subnet_cidr_pair_details: Option<Vec<Ipv6AddressDetails>>` - Explicit IPv6 address / subnet-CIDR pairs. Leave as `None` (rather than `Some(vec![])` or `Some(vec![Ipv6AddressDetails { .. all None }])`) when you want OCI to auto-assign — sending an empty pair makes OCI skip auto-assignment.

### Ipv6AddressDetails

One entry of `CreateVnicDetails::ipv6_address_ipv6_subnet_cidr_pair_details`.

**Fields:**
- `ipv6_address: Option<String>` - Specific IPv6 address to assign. Must lie within the corresponding subnet's IPv6 CIDR.
- `ipv6_subnet_cidr: Option<String>` - Subnet IPv6 CIDR to pick from. Useful when the subnet has more than one IPv6 prefix; can be left `None` to let OCI choose.

**JSON serialization (important):** the SDK uses underscore-separated Rust field names (e.g. `assign_ipv6_ip`, `ipv6_address`) so that `#[serde(rename_all = "camelCase")]` produces the exact keys the OCI API expects (`assignIpv6Ip`, `ipv6Address`, `ipv6SubnetCidr`, `ipv6AddressIpv6SubnetCidrPairDetails`). Do NOT collapse the underscores — `serde`'s camelCase converter does not capitalize letters that follow a digit, so `assign_ipv6ip` would silently serialize as `assignIpv6ip` and OCI would ignore the field.

### LifecycleState

Instance lifecycle state (enum).

**Variants:**
- `Moving` - Instance is being moved
- `Provisioning` - Instance is being provisioned
- `Running` - Instance is running
- `Starting` - Instance is starting
- `Stopping` - Instance is stopping
- `Stopped` - Instance is stopped
- `CreatingImage` - Creating image from instance
- `Terminating` - Instance is terminating
- `Terminated` - Instance is terminated
