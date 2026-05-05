use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Launch instance request details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchInstanceDetails {
    pub availability_domain: String,
    pub compartment_id: String,
    pub shape: String,
    pub source_details: InstanceSourceDetails,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_vnic_details: Option<CreateVnicDetails>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname_label: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape_config: Option<LaunchInstanceShapeConfigDetails>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freeform_tags: Option<HashMap<String, String>>,
}

/// Instance source details (image or boot volume)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "sourceType")]
pub enum InstanceSourceDetails {
    #[serde(rename = "image", rename_all = "camelCase")]
    Image {
        image_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        boot_volume_size_in_gbs: Option<i64>,
    },
    #[serde(rename = "bootVolume", rename_all = "camelCase")]
    BootVolume {
        boot_volume_id: String,
    },
}

/// VNIC creation details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVnicDetails {
    pub subnet_id: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assign_public_ip: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname_label: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_ip: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assign_ipv6_ip: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6_address_ipv6_subnet_cidr_pair_details: Option<Vec<Ipv6AddressDetails>>,
}

/// IPv6 address details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ipv6AddressDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6_address: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6_subnet_cidr: Option<String>,
}

/// Shape configuration details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchInstanceShapeConfigDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocpus: Option<f32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_in_gbs: Option<f32>,
}

/// Instance information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub compartment_id: String,
    pub availability_domain: String,
    pub lifecycle_state: LifecycleState,
    pub shape: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_created: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freeform_tags: Option<HashMap<String, String>>,
}

/// Instance lifecycle state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LifecycleState {
    Moving,
    Provisioning,
    Running,
    Starting,
    Stopping,
    Stopped,
    CreatingImage,
    Terminating,
    Terminated,
}

/// Availability Domain
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityDomain {
    pub name: String,
    pub compartment_id: String,
}

/// Image information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub id: String,
    pub compartment_id: Option<String>,
    pub display_name: Option<String>,
    pub operating_system: Option<String>,
    pub operating_system_version: Option<String>,
    pub lifecycle_state: Option<String>,
    pub time_created: Option<String>,
}

/// Shape information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shape {
    pub shape: String,
    pub processor_description: Option<String>,
    pub ocpus: Option<f32>,
    pub memory_in_gbs: Option<f32>,
    pub networking_bandwidth_in_gbps: Option<f32>,
}

/// VCN (Virtual Cloud Network)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vcn {
    pub id: String,
    pub compartment_id: String,
    pub display_name: Option<String>,
    pub cidr_block: Option<String>,
    pub lifecycle_state: String,
}

/// Subnet
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subnet {
    pub id: String,
    pub compartment_id: String,
    pub vcn_id: String,
    pub display_name: Option<String>,
    pub cidr_block: Option<String>,
    pub ipv6_cidr_block: Option<String>,
    pub lifecycle_state: String,
    pub availability_domain: Option<String>,
}

/// VNIC attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VnicAttachment {
    pub id: String,
    pub compartment_id: String,
    pub instance_id: String,
    pub lifecycle_state: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub vnic_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nic_index: Option<i32>,
}

/// VNIC information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vnic {
    pub id: String,
    pub compartment_id: String,
    pub lifecycle_state: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_ip: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_ip: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<bool>,
}

/// Private IP information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivateIp {
    pub id: String,
    pub compartment_id: String,
    pub ip_address: String,
    pub vnic_id: String,
    pub is_primary: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Public IP information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicIp {
    pub id: String,
    pub compartment_id: String,
    pub ip_address: String,
    pub lifecycle_state: String,
    pub lifetime: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_ip_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Public IP creation details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePublicIpDetails {
    pub compartment_id: String,
    pub lifetime: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_ip_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}
