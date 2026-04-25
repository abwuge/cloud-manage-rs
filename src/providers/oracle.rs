use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use tokio::time::sleep;

use oci_rust_sdk::auth::FileConfigProvider;
use oci_rust_sdk::compute::ComputeClient;
use oci_rust_sdk::compute::models::{
    CreateVnicDetails, InstanceSourceDetails, Ipv6AddressDetails, LaunchInstanceDetails,
    LaunchInstanceShapeConfigDetails, LifecycleState,
};

use crate::config::InstanceConfigFile;

#[derive(Debug, Clone, Copy)]
pub enum AlwaysFreeInstanceType {
    AmdMicro,
    ArmFlex { ocpus: u8, memory_gb: u8 },
}

impl AlwaysFreeInstanceType {
    pub fn shape(&self) -> &'static str {
        match self {
            Self::AmdMicro => "VM.Standard.E2.1.Micro",
            Self::ArmFlex { .. } => "VM.Standard.A1.Flex",
        }
    }
    
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::AmdMicro => Ok(()),
            Self::ArmFlex { ocpus, memory_gb } => {
                if *ocpus == 0 || *ocpus > 4 {
                    return Err("ARM Flex OCPU must be between 1-4".to_string());
                }
                if *memory_gb < 6 || *memory_gb > 24 {
                    return Err("ARM Flex memory must be between 6-24 GB".to_string());
                }
                let expected_memory = *ocpus as u8 * 6;
                if *memory_gb != expected_memory {
                    return Err(format!(
                        "ARM Flex memory mismatch: {} OCPU requires {} GB memory",
                        ocpus, expected_memory
                    ));
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstanceConfig {
    pub instance_type: AlwaysFreeInstanceType,
    pub display_name: String,
    pub assign_public_ip: bool,
    pub boot_volume_size_gb: Option<i64>,
    pub tags: Option<HashMap<String, String>>,
}

impl InstanceConfig {
    pub fn amd_micro(display_name: impl Into<String>) -> Self {
        Self {
            instance_type: AlwaysFreeInstanceType::AmdMicro,
            display_name: display_name.into(),
            assign_public_ip: true,
            boot_volume_size_gb: Some(47),
            tags: None,
        }
    }
    
    pub fn arm_flex(display_name: impl Into<String>, ocpus: u8, memory_gb: u8) -> Self {
        Self {
            instance_type: AlwaysFreeInstanceType::ArmFlex { ocpus, memory_gb },
            display_name: display_name.into(),
            assign_public_ip: true,
            boot_volume_size_gb: Some(47),
            tags: None,
        }
    }
    
    pub fn with_public_ip(mut self, assign: bool) -> Self {
        self.assign_public_ip = assign;
        self
    }
    
    pub fn with_boot_volume_size(mut self, size_gb: i64) -> Self {
        self.boot_volume_size_gb = Some(size_gb);
        self
    }
    
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }
}

pub struct OracleInstanceCreator {
    config_path: String,
    instance_config: InstanceConfigFile,
}

impl OracleInstanceCreator {
    pub fn from_config(
        oci_config_path: impl Into<String>,
        instance_config: InstanceConfigFile,
    ) -> Self {
        Self {
            config_path: oci_config_path.into(),
            instance_config,
        }
    }
    
    pub async fn create_instance(
        &self,
        config: &InstanceConfig,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        config.instance_type.validate()?;

        let auth_config = FileConfigProvider::from_file(Path::new(&self.config_path), "DEFAULT")?;
        let client = ComputeClient::new(&auth_config)?;

        let boot_volume_size = config.boot_volume_size_gb
            .filter(|&size| size >= 50);
        
        let ipv6_details = if self.instance_config.network.assign_ipv6 {
            let ipv6_addr = Ipv6AddressDetails {
                ipv6address: self.instance_config.network.ipv6_address.clone(),
                ipv6subnet_cidr: None,
            };
            Some(vec![ipv6_addr])
        } else {
            None
        };
        
        let launch_details = LaunchInstanceDetails {
            availability_domain: self.instance_config.oracle.availability_domain.clone(),
            compartment_id: self.instance_config.oracle.compartment_id.clone(),
            shape: config.instance_type.shape().to_string(),
            source_details: InstanceSourceDetails::Image {
                image_id: match config.instance_type {
                    AlwaysFreeInstanceType::AmdMicro => self.instance_config.oracle.image_id_amd.clone(),
                    AlwaysFreeInstanceType::ArmFlex { .. } => self.instance_config.oracle.image_id_arm.clone(),
                },
                boot_volume_size_in_gbs: boot_volume_size,
            },
            create_vnic_details: Some(CreateVnicDetails {
                subnet_id: self.instance_config.oracle.subnet_id.clone(),
                assign_public_ip: Some(config.assign_public_ip),
                display_name: Some(format!("{}-vnic", config.display_name)),
                hostname_label: self.instance_config.network.hostname_label.clone(),
                private_ip: self.instance_config.network.private_ip.clone(),
                assign_ipv6ip: Some(self.instance_config.network.assign_ipv6),
                ipv6address_ipv6subnet_cidr_pair_details: ipv6_details,
            }),
            display_name: Some(config.display_name.clone()),
            hostname_label: None,
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("ssh_authorized_keys".to_string(), self.instance_config.oracle.ssh_public_key.clone());
                Some(metadata)
            },
            shape_config: match config.instance_type {
                AlwaysFreeInstanceType::AmdMicro => None,
                AlwaysFreeInstanceType::ArmFlex { ocpus, memory_gb } => {
                    Some(LaunchInstanceShapeConfigDetails {
                        ocpus: Some(ocpus as f32),
                        memory_in_gbs: Some(memory_gb as f32),
                    })
                }
            },
            freeform_tags: config.tags.clone(),
        };
        
        let instance = client.launch_instance(&launch_details).await?;
        
        Ok(instance.id)
    }
    
    pub async fn wait_for_running(
        &self,
        instance_id: &str,
        max_wait_seconds: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let auth_config = FileConfigProvider::from_file(Path::new(&self.config_path), "DEFAULT")?;
        let client = ComputeClient::new(&auth_config)?;

        let start = Instant::now();
        let max_duration = Duration::from_secs(max_wait_seconds);
        
        loop {
            if start.elapsed() > max_duration {
                return Err("Instance start timeout".into());
            }
            
            let instance = client.get_instance(instance_id).await?;
            
            match instance.lifecycle_state {
                LifecycleState::Running => return Ok(()),
                LifecycleState::Terminated | LifecycleState::Terminating => {
                    return Err("Instance terminated".into());
                }
                _ => sleep(Duration::from_secs(5)).await,
            }
        }
    }
    
    pub async fn create_and_wait(
        &self,
        config: &InstanceConfig,
        max_wait_seconds: u64,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let instance_id = self.create_instance(config).await?;
        self.wait_for_running(&instance_id, max_wait_seconds).await?;
        Ok(instance_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_instance_type_validation() {
        assert!(AlwaysFreeInstanceType::AmdMicro.validate().is_ok());
        
        assert!(AlwaysFreeInstanceType::ArmFlex { ocpus: 1, memory_gb: 6 }.validate().is_ok());
        assert!(AlwaysFreeInstanceType::ArmFlex { ocpus: 2, memory_gb: 12 }.validate().is_ok());
        assert!(AlwaysFreeInstanceType::ArmFlex { ocpus: 4, memory_gb: 24 }.validate().is_ok());
        
        assert!(AlwaysFreeInstanceType::ArmFlex { ocpus: 0, memory_gb: 6 }.validate().is_err());
        assert!(AlwaysFreeInstanceType::ArmFlex { ocpus: 5, memory_gb: 30 }.validate().is_err());
        assert!(AlwaysFreeInstanceType::ArmFlex { ocpus: 2, memory_gb: 10 }.validate().is_err());
    }
    
    #[test]
    fn test_instance_config_builder() {
        let config = InstanceConfig::amd_micro("test-instance")
            .with_public_ip(false)
            .with_boot_volume_size(50)
            .with_tag("env", "test");
        
        assert_eq!(config.display_name, "test-instance");
        assert!(!config.assign_public_ip);
        assert_eq!(config.boot_volume_size_gb, Some(50));
        assert!(config.tags.is_some());
    }
}
