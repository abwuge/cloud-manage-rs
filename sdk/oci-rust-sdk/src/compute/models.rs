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
    #[serde(rename = "image")]
    Image {
        image_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        boot_volume_size_in_gbs: Option<i64>,
    },
    #[serde(rename = "bootVolume")]
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
    pub region: String,
    
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
