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
- `cidr_block: Option<String>` - CIDR block
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
- `display_name: Option<String>` - Display name
- `shape: String` - Instance shape
- `region: String` - Region identifier
- `time_created: String` - Creation timestamp

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

Boot source configuration (enum).

**Variants:**
- `Image { image_id: String, boot_volume_size_in_gbs: Option<i32> }` - Boot from image
- `BootVolume { boot_volume_id: String }` - Boot from existing volume

### CreateVnicDetails

Network interface configuration.

**Fields:**
- `subnet_id: String` - Target subnet OCID
- `assign_public_ip: Option<bool>` - Assign public IP
- `display_name: Option<String>` - Display name
- `hostname_label: Option<String>` - Hostname
- `private_ip: Option<String>` - Specific private IP

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
