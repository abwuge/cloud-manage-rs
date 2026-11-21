use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use futures::future::join_all;
use tokio::time::sleep;

use oci_rust_sdk::compute::ComputeClient;
use oci_rust_sdk::compute::models::{
    CreatePublicIpDetails, CreateVnicDetails, Instance, InstanceSourceDetails, Ipv6AddressDetails,
    LaunchInstanceDetails, LaunchInstanceShapeConfigDetails, LifecycleState, PrivateIp, PublicIp,
    VnicAttachment,
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
    pub new_public_ip_id: String,
    pub vnic_id: String,
    pub private_ip_id: String,
}

#[derive(Debug, Clone)]
pub struct VnicIpv4Target {
    pub vnic_id: String,
    pub vnic_display_name: Option<String>,
    pub is_primary_vnic: bool,
    pub nic_index: Option<i32>,
    pub subnet_id: Option<String>,
    pub mac_address: Option<String>,
    pub private_ip_id: String,
    pub private_ip: String,
    pub is_primary_private_ip: bool,
    pub public_ip_id: Option<String>,
    pub public_ip: Option<String>,
    pub public_ip_lifetime: Option<String>,
    pub public_ip_error: Option<String>,
    pub ipv6_addresses: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct InstanceNetworkSummary {
    pub instance_id: String,
    pub display_name: Option<String>,
    pub shape: String,
    pub ocpus: Option<f32>,
    pub memory_in_gbs: Option<f32>,
    pub availability_domain: String,
    pub region: Option<String>,
    pub lifecycle_state: LifecycleState,
    pub compartment_id: String,
    pub time_created: Option<String>,
    pub vnics: Vec<VnicIpv4Target>,
}

impl InstanceNetworkSummary {
    pub fn primary_vnic(&self) -> Option<&VnicIpv4Target> {
        self.vnics
            .iter()
            .find(|v| v.is_primary_vnic && v.is_primary_private_ip)
            .or_else(|| self.vnics.iter().find(|v| v.is_primary_vnic))
            .or_else(|| self.vnics.first())
    }

    pub fn select_target(
        &self,
        vnic_id: Option<&str>,
        private_ip_id: Option<&str>,
    ) -> Result<PublicIpv4Target, Box<dyn std::error::Error + Send + Sync>> {
        let selected = if let Some(pid) = private_ip_id {
            self.vnics
                .iter()
                .find(|v| v.private_ip_id == pid)
                .ok_or_else(|| format!("private IP {} not found on instance {}", pid, self.instance_id))?
        } else if let Some(vid) = vnic_id {
            self.vnics
                .iter()
                .find(|v| v.vnic_id == vid && v.is_primary_private_ip)
                .or_else(|| self.vnics.iter().find(|v| v.vnic_id == vid))
                .ok_or_else(|| format!("VNIC {} not found on instance {}", vid, self.instance_id))?
        } else {
            self.primary_vnic()
                .ok_or_else(|| format!("no attached VNIC found on instance {}", self.instance_id))?
        };

        Ok(self.to_public_ipv4_target(selected))
    }

    pub fn to_public_ipv4_targets(&self) -> Vec<PublicIpv4Target> {
        self.vnics
            .iter()
            .map(|v| self.to_public_ipv4_target(v))
            .collect()
    }

    fn to_public_ipv4_target(&self, vnic: &VnicIpv4Target) -> PublicIpv4Target {
        PublicIpv4Target {
            instance_id: self.instance_id.clone(),
            display_name: self.display_name.clone(),
            lifecycle_state: self.lifecycle_state.clone(),
            vnic_id: vnic.vnic_id.clone(),
            vnic_display_name: vnic.vnic_display_name.clone(),
            is_primary_vnic: vnic.is_primary_vnic,
            nic_index: vnic.nic_index,
            private_ip_id: vnic.private_ip_id.clone(),
            private_ip: vnic.private_ip.clone(),
            is_primary_private_ip: vnic.is_primary_private_ip,
            public_ip_id: vnic.public_ip_id.clone(),
            public_ip: vnic.public_ip.clone(),
            public_ip_lifetime: vnic.public_ip_lifetime.clone(),
            public_ip_error: vnic.public_ip_error.clone(),
            compartment_id: self.compartment_id.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PublicIpv4Target {
    pub instance_id: String,
    pub display_name: Option<String>,
    pub lifecycle_state: LifecycleState,
    pub vnic_id: String,
    pub vnic_display_name: Option<String>,
    pub is_primary_vnic: bool,
    pub nic_index: Option<i32>,
    pub private_ip_id: String,
    pub private_ip: String,
    pub is_primary_private_ip: bool,
    pub public_ip_id: Option<String>,
    pub public_ip: Option<String>,
    pub public_ip_lifetime: Option<String>,
    pub public_ip_error: Option<String>,
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

    pub async fn list_instance_networks(
        &self,
    ) -> Result<Vec<InstanceNetworkSummary>, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.make_client()?;
        let mut instances = client
            .list_instances(&self.instance_config.oracle.compartment_id)
            .await?;
        instances.retain(|instance| instance.lifecycle_state != LifecycleState::Terminated);

        let futures = instances
            .into_iter()
            .map(|instance| self.instance_network_summary(&client, instance));
        let results = join_all(futures).await;

        let mut summaries = Vec::new();
        for result in results {
            match result {
                Ok(summary) => summaries.push(summary),
                Err(error) => {
                    eprintln!(
                        "⚠️  Skipping instance with incomplete network info: {}",
                        error
                    );
                }
            }
        }

        sort_instance_summaries(&mut summaries);
        Ok(summaries)
    }

    pub async fn list_public_ipv4_targets(
        &self,
    ) -> Result<Vec<PublicIpv4Target>, Box<dyn std::error::Error + Send + Sync>> {
        let summaries = self.list_instance_networks().await?;
        let targets = summaries
            .iter()
            .flat_map(InstanceNetworkSummary::to_public_ipv4_targets)
            .collect();
        Ok(targets)
    }

    pub async fn instance_network_for_instance_id(
        &self,
        instance_id: &str,
    ) -> Result<InstanceNetworkSummary, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.make_client()?;
        let instance = client.get_instance(instance_id).await?;
        self.instance_network_summary(&client, instance).await
    }

    pub async fn public_ipv4_target_for_instance_id(
        &self,
        instance_id: &str,
        vnic_id: Option<&str>,
        private_ip_id: Option<&str>,
    ) -> Result<PublicIpv4Target, Box<dyn std::error::Error + Send + Sync>> {
        let summary = self.instance_network_for_instance_id(instance_id).await?;
        summary.select_target(vnic_id, private_ip_id)
    }

    pub async fn refresh_public_ipv4_target(
        &self,
        target: &PublicIpv4Target,
    ) -> Result<PublicIpRefreshResult, Box<dyn std::error::Error + Send + Sync>> {
        validate_refresh_target(target)?;

        let client = self.make_client()?;
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
            new_public_ip_id: public_ip.id,
            vnic_id: target.vnic_id.clone(),
            private_ip_id: target.private_ip_id.clone(),
        })
    }

    async fn instance_network_summary(
        &self,
        client: &ComputeClient,
        instance: Instance,
    ) -> Result<InstanceNetworkSummary, Box<dyn std::error::Error + Send + Sync>> {
        let attachments = client
            .list_vnic_attachments(&instance.compartment_id, &instance.id)
            .await?;

        let attached = filter_attached_vnics(&attachments);
        let vnic_results = join_all(
            attached
                .into_iter()
                .map(|attachment| self.vnic_ipv4_targets(client, attachment)),
        )
        .await;

        let mut vnics = Vec::new();
        for result in vnic_results {
            match result {
                Ok(mut items) => vnics.append(&mut items),
                Err(error) => {
                    eprintln!(
                        "⚠️  Skipping VNIC on instance {} due to lookup error: {}",
                        instance.id, error
                    );
                }
            }
        }

        sort_vnic_targets(&mut vnics);

        Ok(InstanceNetworkSummary {
            instance_id: instance.id,
            display_name: instance.display_name,
            shape: instance.shape,
            ocpus: instance.shape_config.as_ref().and_then(|s| s.ocpus),
            memory_in_gbs: instance.shape_config.as_ref().and_then(|s| s.memory_in_gbs),
            availability_domain: instance.availability_domain,
            region: instance.region,
            lifecycle_state: instance.lifecycle_state,
            compartment_id: instance.compartment_id,
            time_created: instance.time_created,
            vnics,
        })
    }

    async fn vnic_ipv4_targets(
        &self,
        client: &ComputeClient,
        attachment: &VnicAttachment,
    ) -> Result<Vec<VnicIpv4Target>, Box<dyn std::error::Error + Send + Sync>> {
        let vnic_id = attachment
            .vnic_id
            .as_deref()
            .ok_or("VNIC attachment does not include a VNIC id")?;

        let (vnic_res, private_ips_res) =
            tokio::join!(client.get_vnic(vnic_id), client.list_private_ips(vnic_id));
        let vnic = vnic_res?;
        let mut private_ips = private_ips_res?;

        if private_ips.is_empty() {
            return Err(format!("no private IPv4 found on VNIC {}", vnic_id).into());
        }

        private_ips.sort_by(|a, b| {
            (!a.is_primary)
                .cmp(&(!b.is_primary))
                .then_with(|| a.ip_address.cmp(&b.ip_address))
        });

        let is_primary_vnic = vnic.is_primary.unwrap_or(false);
        let subnet_id = vnic.subnet_id.clone().or_else(|| attachment.subnet_id.clone());
        let mac_address = vnic.mac_address.clone();
        let vnic_display_name = vnic
            .display_name
            .clone()
            .or_else(|| attachment.display_name.clone());
        let vnic_public_ip = vnic.public_ip.clone();
        let ipv6_addresses = vnic.ipv6_addresses.clone().unwrap_or_default();

        let ip_lookups = join_all(private_ips.into_iter().map(|private_ip| {
            let vnic_pub = if private_ip.is_primary {
                vnic_public_ip.clone()
            } else {
                None
            };
            async move {
                let (public_ip, public_ip_error) =
                    resolve_public_ip(client, &private_ip, vnic_pub.as_deref()).await;
                (private_ip, vnic_pub, public_ip, public_ip_error)
            }
        }))
        .await;

        let mut targets = Vec::new();
        for (private_ip, vnic_pub, public_ip, public_ip_error) in ip_lookups {
            let resolved_ip = public_ip
                .as_ref()
                .map(|ip| ip.ip_address.clone())
                .or(vnic_pub);
            if !private_ip.is_primary && resolved_ip.is_none() && public_ip_error.is_none() {
                continue;
            }

            targets.push(VnicIpv4Target {
                vnic_id: vnic.id.clone(),
                vnic_display_name: vnic_display_name.clone(),
                is_primary_vnic,
                nic_index: attachment.nic_index,
                subnet_id: subnet_id.clone().or(private_ip.subnet_id),
                mac_address: mac_address.clone(),
                private_ip_id: private_ip.id,
                private_ip: private_ip.ip_address,
                is_primary_private_ip: private_ip.is_primary,
                public_ip_id: public_ip.as_ref().map(|ip| ip.id.clone()),
                public_ip: resolved_ip,
                public_ip_lifetime: public_ip.as_ref().map(|ip| ip.lifetime.clone()),
                public_ip_error,
                ipv6_addresses: if private_ip.is_primary {
                    ipv6_addresses.clone()
                } else {
                    Vec::new()
                },
            });
        }

        Ok(targets)
    }
}

async fn resolve_public_ip(
    client: &ComputeClient,
    private_ip: &PrivateIp,
    vnic_public_ip: Option<&str>,
) -> (Option<PublicIp>, Option<String>) {
    match client.get_public_ip_by_private_ip_id(&private_ip.id).await {
        Ok(Some(public_ip)) => (Some(public_ip), None),
        Ok(None) => match vnic_public_ip {
            Some(ip_address) => match client.get_public_ip_by_ip_address(ip_address).await {
                Ok(public_ip) => (Some(public_ip), None),
                Err(error) => (None, Some(format!("public IP id lookup failed: {}", error))),
            },
            None => (None, None),
        },
        Err(primary_err) => match vnic_public_ip {
            Some(ip_address) => match client.get_public_ip_by_ip_address(ip_address).await {
                Ok(public_ip) => (Some(public_ip), None),
                Err(fallback_err) => (
                    None,
                    Some(format!("public IP id lookup failed: {}", fallback_err)),
                ),
            },
            None => (
                None,
                Some(format!("public IP lookup failed: {}", primary_err)),
            ),
        },
    }
}

fn filter_attached_vnics(attachments: &[VnicAttachment]) -> Vec<&VnicAttachment> {
    let mut seen = HashSet::new();
    attachments
        .iter()
        .filter(|a| a.lifecycle_state.eq_ignore_ascii_case("ATTACHED"))
        .filter(|a| match a.vnic_id.as_deref() {
            Some(id) => seen.insert(id.to_string()),
            None => false,
        })
        .collect()
}

fn sort_vnic_targets(targets: &mut [VnicIpv4Target]) {
    targets.sort_by(|a, b| {
        (!a.is_primary_vnic)
            .cmp(&(!b.is_primary_vnic))
            .then_with(|| {
                a.nic_index
                    .unwrap_or(i32::MAX)
                    .cmp(&b.nic_index.unwrap_or(i32::MAX))
            })
            .then_with(|| (!a.is_primary_private_ip).cmp(&(!b.is_primary_private_ip)))
            .then_with(|| a.private_ip.cmp(&b.private_ip))
            .then_with(|| a.vnic_id.cmp(&b.vnic_id))
    });
}

fn sort_instance_summaries(summaries: &mut [InstanceNetworkSummary]) {
    summaries.sort_by(|a, b| {
        let a_rank = if a.lifecycle_state == LifecycleState::Running { 0 } else { 1 };
        let b_rank = if b.lifecycle_state == LifecycleState::Running { 0 } else { 1 };
        a_rank
            .cmp(&b_rank)
            .then_with(|| {
                a.display_name
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .cmp(&b.display_name.as_deref().unwrap_or("").to_lowercase())
            })
            .then_with(|| a.instance_id.cmp(&b.instance_id))
    });
}

fn validate_refresh_target(
    target: &PublicIpv4Target,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if target.lifecycle_state != LifecycleState::Running {
        return Err(format!(
            "instance must be RUNNING before refreshing public IPv4 (current: {:?})",
            target.lifecycle_state
        )
        .into());
    }
    if !target.is_primary_private_ip {
        return Err(
            "ephemeral public IPv4 refresh is only supported on a VNIC's primary private IP".into(),
        );
    }
    if target
        .public_ip_lifetime
        .as_deref()
        .is_some_and(|l| l.eq_ignore_ascii_case("RESERVED"))
    {
        return Err(
            "cannot refresh a RESERVED public IPv4 automatically; only EPHEMERAL public IPs can be rotated"
                .into(),
        );
    }
    if target.public_ip.is_some() && target.public_ip_id.is_none() {
        return Err(
            "current public IPv4 was found, but its OCI public IP id could not be loaded; refusing to refresh without a deletable public IP id"
                .into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vnic(
        vnic_id: &str,
        is_primary_vnic: bool,
        nic_index: Option<i32>,
        private_ip_id: &str,
        private_ip: &str,
        is_primary_private_ip: bool,
        public_ip: Option<&str>,
        lifetime: Option<&str>,
    ) -> VnicIpv4Target {
        VnicIpv4Target {
            vnic_id: vnic_id.to_string(),
            vnic_display_name: Some(format!("{}-name", vnic_id)),
            is_primary_vnic,
            nic_index,
            subnet_id: Some("ocid1.subnet.test".to_string()),
            mac_address: Some("02:00:17:00:00:01".to_string()),
            private_ip_id: private_ip_id.to_string(),
            private_ip: private_ip.to_string(),
            is_primary_private_ip,
            public_ip_id: public_ip.map(|_| format!("ocid1.publicip.{}", private_ip_id)),
            public_ip: public_ip.map(str::to_string),
            public_ip_lifetime: lifetime.map(str::to_string),
            public_ip_error: None,
            ipv6_addresses: Vec::new(),
        }
    }

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

    #[test]
    fn test_sort_and_select_primary_vnic_when_secondary_shares_nic_zero() {
        // OCI often returns the newest (secondary) VNIC first, both on nic_index 0.
        let mut vnics = vec![
            sample_vnic(
                "vnic-secondary",
                false,
                Some(0),
                "pip-sec",
                "10.0.0.20",
                true,
                Some("129.1.1.2"),
                Some("EPHEMERAL"),
            ),
            sample_vnic(
                "vnic-primary",
                true,
                Some(0),
                "pip-pri",
                "10.0.0.10",
                true,
                Some("140.1.1.1"),
                Some("EPHEMERAL"),
            ),
        ];

        sort_vnic_targets(&mut vnics);
        assert_eq!(vnics[0].vnic_id, "vnic-primary");
        assert!(vnics[0].is_primary_vnic);
        assert_eq!(vnics[1].vnic_id, "vnic-secondary");
        assert!(!vnics[1].is_primary_vnic);

        let summary = InstanceNetworkSummary {
            instance_id: "ocid1.instance.arm4c24g".to_string(),
            display_name: Some("arm4c24g".to_string()),
            shape: "VM.Standard.A1.Flex".to_string(),
            ocpus: Some(4.0),
            memory_in_gbs: Some(24.0),
            availability_domain: "AD-1".to_string(),
            region: Some("ap-tokyo-1".to_string()),
            lifecycle_state: LifecycleState::Running,
            compartment_id: "ocid1.compartment.test".to_string(),
            time_created: None,
            vnics,
        };

        // Default selection picks primary VNIC.
        let default_target = summary.select_target(None, None).unwrap();
        assert_eq!(default_target.vnic_id, "vnic-primary");
        assert_eq!(default_target.public_ip.as_deref(), Some("140.1.1.1"));

        // Explicit selection by vnic_id picks secondary VNIC.
        let sec_target = summary.select_target(Some("vnic-secondary"), None).unwrap();
        assert_eq!(sec_target.vnic_id, "vnic-secondary");
        assert_eq!(sec_target.public_ip.as_deref(), Some("129.1.1.2"));

        // Explicit selection by private_ip_id picks exact IP.
        let pip_target = summary.select_target(None, Some("pip-sec")).unwrap();
        assert_eq!(pip_target.private_ip_id, "pip-sec");
    }

    #[test]
    fn test_filter_attached_vnics_excludes_detached_and_duplicates() {
        let attachments = vec![
            VnicAttachment {
                id: "att-1".to_string(),
                compartment_id: "comp".to_string(),
                instance_id: "inst".to_string(),
                lifecycle_state: "DETACHED".to_string(),
                vnic_id: Some("vnic-old".to_string()),
                subnet_id: None,
                display_name: None,
                nic_index: Some(0),
                vlan_tag: None,
            },
            VnicAttachment {
                id: "att-2".to_string(),
                compartment_id: "comp".to_string(),
                instance_id: "inst".to_string(),
                lifecycle_state: "ATTACHED".to_string(),
                vnic_id: Some("vnic-sec".to_string()),
                subnet_id: None,
                display_name: None,
                nic_index: Some(0),
                vlan_tag: None,
            },
            VnicAttachment {
                id: "att-3".to_string(),
                compartment_id: "comp".to_string(),
                instance_id: "inst".to_string(),
                lifecycle_state: "ATTACHED".to_string(),
                vnic_id: Some("vnic-pri".to_string()),
                subnet_id: None,
                display_name: None,
                nic_index: Some(0),
                vlan_tag: None,
            },
        ];

        let filtered = filter_attached_vnics(&attachments);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].vnic_id.as_deref(), Some("vnic-sec"));
        assert_eq!(filtered[1].vnic_id.as_deref(), Some("vnic-pri"));
    }

    #[test]
    fn test_validate_refresh_target_guards_reserved_and_secondary_private_ip() {
        let mut target = PublicIpv4Target {
            instance_id: "inst".to_string(),
            display_name: Some("arm4c24g".to_string()),
            lifecycle_state: LifecycleState::Running,
            vnic_id: "vnic-1".to_string(),
            vnic_display_name: None,
            is_primary_vnic: true,
            nic_index: Some(0),
            private_ip_id: "pip-1".to_string(),
            private_ip: "10.0.0.10".to_string(),
            is_primary_private_ip: true,
            public_ip_id: Some("pub-1".to_string()),
            public_ip: Some("140.1.1.1".to_string()),
            public_ip_lifetime: Some("EPHEMERAL".to_string()),
            public_ip_error: None,
            compartment_id: "comp".to_string(),
        };
        assert!(validate_refresh_target(&target).is_ok());

        target.public_ip_lifetime = Some("RESERVED".to_string());
        assert!(validate_refresh_target(&target).is_err());

        target.public_ip_lifetime = Some("EPHEMERAL".to_string());
        target.is_primary_private_ip = false;
        assert!(validate_refresh_target(&target).is_err());
    }
}
