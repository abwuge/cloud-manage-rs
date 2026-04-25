use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfigFile {
    pub oracle: OracleConfig,
    pub instance: InstanceSettings,
    pub network: NetworkSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleConfig {
    pub compartment_id: String,
    pub availability_domain: String,
    pub subnet_id: String,
    pub image_id_amd: String,
    pub image_id_arm: String,
    pub ssh_public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSettings {
    pub instance_type: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_ocpus: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_memory_gb: Option<u8>,
    pub boot_volume_size_gb: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub assign_public_ip: bool,
    #[serde(default)]
    pub assign_ipv6: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname_label: Option<String>,
}

impl Default for InstanceConfigFile {
    fn default() -> Self {
        Self {
            oracle: OracleConfig {
                compartment_id: "ocid1.compartment.oc1..your-compartment-id".to_string(),
                availability_domain: "your-region-AD-1".to_string(),
                subnet_id: "ocid1.subnet.oc1..your-subnet-id".to_string(),
                image_id_amd: "ocid1.image.oc1..your-amd-image-id".to_string(),
                image_id_arm: "ocid1.image.oc1..your-arm-image-id".to_string(),
                ssh_public_key: "ssh-rsa AAAAB3NzaC1yc2E... your-ssh-public-key".to_string(),
            },
            instance: InstanceSettings {
                instance_type: "amd".to_string(),
                display_name: "my-instance".to_string(),
                arm_ocpus: Some(2),
                arm_memory_gb: Some(12),
                boot_volume_size_gb: 47,
            },
            network: NetworkSettings {
                assign_public_ip: true,
                assign_ipv6: false,
                private_ip: None,
                ipv6_address: None,
                hostname_label: None,
            },
        }
    }
}

impl InstanceConfigFile {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
    
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let content = toml::to_string_pretty(self)?;
        
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(path, content)?;
        Ok(())
    }
    
    pub fn exists(path: impl AsRef<Path>) -> bool {
        path.as_ref().exists()
    }
}
