use super::{AuthError, ConfigurationProvider, Result};
use configparser::ini::Ini;
use std::fs;
use std::path::{Path, PathBuf};

/// File-based configuration provider that reads from OCI config file
pub struct FileConfigProvider {
    user_id: String,
    tenancy_id: String,
    fingerprint: String,
    private_key: String,
    region: String,
    passphrase: Option<String>,
}

impl FileConfigProvider {
    /// Create a new provider from the default config file (~/.oci/config) and DEFAULT profile
    pub fn new() -> Result<Self> {
        Self::from_profile("DEFAULT")
    }

    /// Create a new provider from the default config file with a specific profile
    pub fn from_profile(profile: &str) -> Result<Self> {
        let config_path = Self::default_config_path()?;
        Self::from_file(&config_path, profile)
    }

    /// Create a new provider from a specific config file and profile
    pub fn from_file(path: &Path, profile: &str) -> Result<Self> {
        let mut conf = Ini::new();
        conf.load(path).map_err(|e| {
            AuthError::ConfigNotFound(format!("Failed to load config file: {}", e))
        })?;

        let user_id = conf
            .get(profile, "user")
            .ok_or_else(|| AuthError::InvalidConfig("Missing 'user' field".to_string()))?;

        let tenancy_id = conf
            .get(profile, "tenancy")
            .ok_or_else(|| AuthError::InvalidConfig("Missing 'tenancy' field".to_string()))?;

        let fingerprint = conf
            .get(profile, "fingerprint")
            .ok_or_else(|| AuthError::InvalidConfig("Missing 'fingerprint' field".to_string()))?;

        let region = conf
            .get(profile, "region")
            .ok_or_else(|| AuthError::InvalidConfig("Missing 'region' field".to_string()))?;

        let key_file = conf
            .get(profile, "key_file")
            .ok_or_else(|| AuthError::InvalidConfig("Missing 'key_file' field".to_string()))?;

        let key_path = Self::expand_path(&key_file);
        let private_key = fs::read_to_string(&key_path).map_err(|e| {
            AuthError::ConfigNotFound(format!("Failed to read private key file: {}", e))
        })?;

        let passphrase = conf.get(profile, "pass_phrase");

        Ok(Self {
            user_id,
            tenancy_id,
            fingerprint,
            private_key,
            region,
            passphrase,
        })
    }

    fn default_config_path() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| AuthError::ConfigNotFound("Cannot determine home directory".to_string()))?;

        Ok(PathBuf::from(home).join(".oci").join("config"))
    }

    fn expand_path(path: &str) -> PathBuf {
        if path.starts_with("~/") {
            if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
                return PathBuf::from(home).join(&path[2..]);
            }
        }
        PathBuf::from(path)
    }
}

impl ConfigurationProvider for FileConfigProvider {
    fn user_id(&self) -> Result<String> {
        Ok(self.user_id.clone())
    }

    fn tenancy_id(&self) -> Result<String> {
        Ok(self.tenancy_id.clone())
    }

    fn fingerprint(&self) -> Result<String> {
        Ok(self.fingerprint.clone())
    }

    fn private_key(&self) -> Result<String> {
        Ok(self.private_key.clone())
    }

    fn region(&self) -> Result<String> {
        Ok(self.region.clone())
    }

    fn passphrase(&self) -> Result<Option<String>> {
        Ok(self.passphrase.clone())
    }
}
