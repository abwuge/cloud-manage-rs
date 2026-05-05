use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::time::sleep;

use oci_rust_sdk::compute::ComputeClient;
use oci_rust_sdk::compute::models::{
    CreatePublicIpDetails, CreateVnicDetails, Instance, InstanceSourceDetails, Ipv6AddressDetails,
    LaunchInstanceDetails, LaunchInstanceShapeConfigDetails, LifecycleState,
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
        self.tags
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }
}

pub struct OracleInstanceCreator {
    instance_config: InstanceConfigFile,
}

#[derive(Debug, Clone)]
pub struct PublicIpRefreshResult {
    pub old_public_ip: Option<String>,
    pub new_public_ip: String,
}

#[derive(Debug, Clone)]
pub struct PublicIpv4Target {
    pub instance_id: String,
    pub display_name: Option<String>,
    pub lifecycle_state: LifecycleState,
    pub private_ip_id: String,
    pub public_ip_id: Option<String>,
    pub public_ip: Option<String>,
    pub public_ip_error: Option<String>,
    pub vnic_display_name: Option<String>,
    pub compartment_id: String,
}

impl OracleInstanceCreator {
    pub fn new(instance_config: InstanceConfigFile) -> Self {
        Self { instance_config }
    }

    fn make_client(&self) -> Result<ComputeClient, Box<dyn std::error::Error + Send + Sync>> {
        ComputeClient::new(&self.instance_config.oci)
    }

    pub async fn create_instance(
        &self,
        config: &InstanceConfig,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        config.instance_type.validate()?;
        let client = self.make_client()?;

        let boot_volume_size = config.boot_volume_size_gb.filter(|&size| size >= 50);

        let ipv6_details = if self.instance_config.network.assign_ipv6 {
            self.instance_config
                .network
                .ipv6_address
                .as_ref()
                .map(|addr| {
                    vec![Ipv6AddressDetails {
                        ipv6_address: Some(addr.clone()),
                        ipv6_subnet_cidr: None,
                    }]
                })
        } else {
            None
        };

        let launch_details = LaunchInstanceDetails {
            availability_domain: self.instance_config.oracle.availability_domain.clone(),
            compartment_id: self.instance_config.oracle.compartment_id.clone(),
            shape: config.instance_type.shape().to_string(),
            source_details: InstanceSourceDetails::Image {
                image_id: match config.instance_type {
                    AlwaysFreeInstanceType::AmdMicro => {
                        self.instance_config.oracle.image_id_amd.clone()
                    }
                    AlwaysFreeInstanceType::ArmFlex { .. } => {
                        self.instance_config.oracle.image_id_arm.clone()
                    }
                },
                boot_volume_size_in_gbs: boot_volume_size,
            },
            create_vnic_details: Some(CreateVnicDetails {
                subnet_id: self.instance_config.oracle.subnet_id.clone(),
                assign_public_ip: Some(config.assign_public_ip),
                display_name: Some(format!("{}-vnic", config.display_name)),
                hostname_label: self.instance_config.network.hostname_label.clone(),
                private_ip: self.instance_config.network.private_ip.clone(),
                assign_ipv6_ip: Some(self.instance_config.network.assign_ipv6),
                ipv6_address_ipv6_subnet_cidr_pair_details: ipv6_details,
            }),
            display_name: Some(config.display_name.clone()),
            hostname_label: None,
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert(
                    "ssh_authorized_keys".to_string(),
                    self.instance_config.oracle.ssh_public_key.clone(),
                );
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
        let client = self.make_client()?;

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
        self.wait_for_running(&instance_id, max_wait_seconds)
            .await?;
        Ok(instance_id)
    }

    pub async fn list_public_ipv4_targets(
        &self,
    ) -> Result<Vec<PublicIpv4Target>, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.make_client()?;
        let mut instances = client
            .list_instances(&self.instance_config.oracle.compartment_id)
            .await?;
        instances.retain(|instance| instance.lifecycle_state != LifecycleState::Terminated);

        let mut targets = Vec::new();
        for instance in instances {
            match self
                .public_ipv4_target_for_instance(&client, instance)
                .await
            {
                Ok(target) => targets.push(target),
                Err(error) => {
                    eprintln!(
                        "⚠️  Skipping instance with incomplete network info: {}",
                        error
                    );
                }
            }
        }

        Ok(targets)
    }

    pub async fn public_ipv4_target_for_instance_id(
        &self,
        instance_id: &str,
    ) -> Result<PublicIpv4Target, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.make_client()?;
        let instance = client.get_instance(instance_id).await?;
        self.public_ipv4_target_for_instance(&client, instance)
            .await
    }

    pub async fn refresh_public_ipv4_target(
        &self,
        target: &PublicIpv4Target,
    ) -> Result<PublicIpRefreshResult, Box<dyn std::error::Error + Send + Sync>> {
        if target.lifecycle_state != LifecycleState::Running {
            return Err(format!(
                "instance must be RUNNING before refreshing public IPv4 (current: {:?})",
                target.lifecycle_state
            )
            .into());
        }

        let client = self.make_client()?;
        if target.public_ip.is_some() && target.public_ip_id.is_none() {
            return Err("current public IPv4 was found, but its OCI public IP id could not be loaded; refusing to refresh without a deletable public IP id".into());
        }
        if let Some(public_ip_id) = &target.public_ip_id {
            client.delete_public_ip(public_ip_id).await?;
        }

        let public_ip = client
            .create_public_ip(&CreatePublicIpDetails {
                compartment_id: target.compartment_id.clone(),
                lifetime: "EPHEMERAL".to_string(),
                private_ip_id: Some(target.private_ip_id.clone()),
                display_name: target
                    .vnic_display_name
                    .as_ref()
                    .map(|name| format!("{}-refreshed-public-ip", name)),
            })
            .await?;

        Ok(PublicIpRefreshResult {
            old_public_ip: target.public_ip.clone(),
            new_public_ip: public_ip.ip_address,
        })
    }

    async fn public_ipv4_target_for_instance(
        &self,
        client: &ComputeClient,
        instance: Instance,
    ) -> Result<PublicIpv4Target, Box<dyn std::error::Error + Send + Sync>> {
        let attachments = client
            .list_vnic_attachments(&instance.compartment_id, &instance.id)
            .await?;
        let attachment = attachments
            .iter()
            .find(|attachment| attachment.nic_index == Some(0) && attachment.vnic_id.is_some())
            .or_else(|| {
                attachments
                    .iter()
                    .find(|attachment| attachment.vnic_id.is_some())
            })
            .ok_or("no VNIC attachment found for instance")?;
        let vnic_id = attachment
            .vnic_id
            .as_deref()
            .ok_or("VNIC attachment does not include a VNIC id")?;

        let vnic = client.get_vnic(vnic_id).await?;
        let private_ips = client.list_private_ips(vnic_id).await?;
        let private_ip = private_ips
            .iter()
            .find(|ip| ip.is_primary)
            .or_else(|| private_ips.first())
            .ok_or("no private IPv4 found on instance VNIC")?;

        let vnic_public_ip = vnic.public_ip.clone();
        let (public_ip, public_ip_error) = match vnic_public_ip.as_deref() {
            Some(ip_address) => match client.get_public_ip_by_ip_address(ip_address).await {
                Ok(public_ip) => (Some(public_ip), None),
                Err(error) => (None, Some(format!("public IP id lookup failed: {}", error))),
            },
            None => (None, None),
        };

        Ok(PublicIpv4Target {
            instance_id: instance.id,
            display_name: instance.display_name,
            lifecycle_state: instance.lifecycle_state,
            private_ip_id: private_ip.id.clone(),
            public_ip_id: public_ip.as_ref().map(|ip| ip.id.clone()),
            public_ip: public_ip
                .as_ref()
                .map(|ip| ip.ip_address.clone())
                .or(vnic_public_ip),
            public_ip_error,
            vnic_display_name: vnic.display_name,
            compartment_id: instance.compartment_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_type_validation() {
        assert!(AlwaysFreeInstanceType::AmdMicro.validate().is_ok());

        assert!(
            AlwaysFreeInstanceType::ArmFlex {
                ocpus: 1,
                memory_gb: 6
            }
            .validate()
            .is_ok()
        );
        assert!(
            AlwaysFreeInstanceType::ArmFlex {
                ocpus: 2,
                memory_gb: 12
            }
            .validate()
            .is_ok()
        );
        assert!(
            AlwaysFreeInstanceType::ArmFlex {
                ocpus: 4,
                memory_gb: 24
            }
            .validate()
            .is_ok()
        );

        assert!(
            AlwaysFreeInstanceType::ArmFlex {
                ocpus: 0,
                memory_gb: 6
            }
            .validate()
            .is_err()
        );
        assert!(
            AlwaysFreeInstanceType::ArmFlex {
                ocpus: 5,
                memory_gb: 30
            }
            .validate()
            .is_err()
        );
        assert!(
            AlwaysFreeInstanceType::ArmFlex {
                ocpus: 2,
                memory_gb: 10
            }
            .validate()
            .is_err()
        );
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
