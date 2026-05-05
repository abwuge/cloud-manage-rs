use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use cloudflare_rust_sdk::auth::{
    AuthError as CloudflareAuthError, ConfigurationProvider as CloudflareConfigurationProvider,
    Result as CloudflareAuthResult,
};
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
    #[serde(default)]
    pub cloudflare: CloudflareConfig,
    pub oracle: OracleConfig,
    pub instance: InstanceSettings,
    pub network: NetworkSettings,
    #[serde(default)]
    pub snipe: SnipeSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnipeSettings {
    pub min_delay_secs: f64,
    pub max_delay_secs: f64,
    pub max_attempts: u32,
}

impl Default for SnipeSettings {
    fn default() -> Self {
        Self {
            min_delay_secs: 5.0,
            max_delay_secs: 30.0,
            max_attempts: 0,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CloudflareConfig {
    #[serde(default)]
    pub api_token: String,
    #[serde(default)]
    pub zone_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_name: Option<String>,
}

impl CloudflareConfigurationProvider for CloudflareConfig {
    fn api_token(&self) -> CloudflareAuthResult<String> {
        if self.api_token.trim().is_empty() {
            Err(CloudflareAuthError::MissingValue("api_token".to_string()))
        } else {
            Ok(self.api_token.clone())
        }
    }

    fn zone_name(&self) -> CloudflareAuthResult<String> {
        if self.zone_name.trim().is_empty() {
            Err(CloudflareAuthError::MissingValue("zone_name".to_string()))
        } else {
            Ok(self.zone_name.clone())
        }
    }
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
            cloudflare: CloudflareConfig {
                api_token: String::new(),
                zone_name: String::new(),
                record_name: None,
            },
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
            snipe: SnipeSettings::default(),
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
}
