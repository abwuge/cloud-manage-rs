use super::Result;

/// Configuration provider trait for OCI authentication
pub trait ConfigurationProvider {
    /// Returns the user OCID
    fn user_id(&self) -> Result<String>;
    
    /// Returns the tenancy OCID
    fn tenancy_id(&self) -> Result<String>;
    
    /// Returns the public key fingerprint
    fn fingerprint(&self) -> Result<String>;
    
    /// Returns the private key in PEM format
    fn private_key(&self) -> Result<String>;
    
    /// Returns the region identifier
    fn region(&self) -> Result<String>;
    
    /// Returns the passphrase for the private key, if any
    fn passphrase(&self) -> Result<Option<String>> {
        Ok(None)
    }
    
    /// Returns the key ID in the format: {tenancy}/{user}/{fingerprint}
    fn key_id(&self) -> Result<String> {
        Ok(format!(
            "{}/{}/{}",
            self.tenancy_id()?,
            self.user_id()?,
            self.fingerprint()?
        ))
    }
}
