use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use oci_rust_sdk::auth::{AuthError, ConfigurationProvider, Result as AuthResult};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OciAuthConfig {
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub fingerprint: String,
    #[serde(default)]
    pub tenancy: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub key_file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
}

impl ConfigurationProvider for OciAuthConfig {
    fn user_id(&self) -> AuthResult<String> {
        Ok(self.user.clone())
    }
    fn tenancy_id(&self) -> AuthResult<String> {
        Ok(self.tenancy.clone())
    }
    fn fingerprint(&self) -> AuthResult<String> {
        Ok(self.fingerprint.clone())
    }
    fn region(&self) -> AuthResult<String> {
        Ok(self.region.clone())
    }
    fn private_key(&self) -> AuthResult<String> {
        fs::read_to_string(&self.key_file).map_err(AuthError::from)
    }
    fn passphrase(&self) -> AuthResult<Option<String>> {
        Ok(self.passphrase.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfigFile {
    #[serde(default)]
    pub oci: OciAuthConfig,
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
            oci: OciAuthConfig::default(),
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
    pub fn load_from_file(
        path: impl AsRef<Path>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    /// One-shot migration from the legacy split files to a single unified TOML.
    /// - `target`: destination unified config (e.g. `./config/config`).
    /// - `legacy_oci_ini`: old OCI auth INI (e.g. `./config/oci_config`).
    /// - `legacy_instance_toml`: old `instance_config.toml`.
    ///
    /// Runs only when `target` is missing AND both legacy files are present.
    /// On success the legacy files are left in place; the caller can delete
    /// them after verifying.
    pub fn migrate_legacy(
        target: impl AsRef<Path>,
        legacy_oci_ini: impl AsRef<Path>,
        legacy_instance_toml: impl AsRef<Path>,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let target = target.as_ref();
        if target.exists() {
            return Ok(false);
        }
        let oci_ini = legacy_oci_ini.as_ref();
        let instance_toml = legacy_instance_toml.as_ref();
        if !oci_ini.exists() || !instance_toml.exists() {
            return Ok(false);
        }

        let oci = parse_oci_ini(oci_ini)?;
        let mut merged: InstanceConfigFile = Self::load_from_file(instance_toml)?;
        merged.oci = oci;
        merged.save_to_file(target)?;
        Ok(true)
    }
}

fn parse_oci_ini(path: &Path) -> Result<OciAuthConfig, Box<dyn std::error::Error + Send + Sync>> {
    let content = fs::read_to_string(path)?;
    let mut cfg = OciAuthConfig::default();
    let mut in_default = false;
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_default = line == "[DEFAULT]";
            continue;
        }
        if !in_default {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim().to_string();
        match k.trim() {
            "user" => cfg.user = v,
            "fingerprint" => cfg.fingerprint = v,
            "tenancy" => cfg.tenancy = v,
            "region" => cfg.region = v,
            "key_file" => cfg.key_file = v,
            "pass_phrase" | "passphrase" => cfg.passphrase = Some(v),
            _ => {}
        }
    }
    Ok(cfg)
}
