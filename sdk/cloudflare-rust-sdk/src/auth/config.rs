use super::{AuthError, Result};

/// Configuration provider trait for Cloudflare API token authentication.
pub trait ConfigurationProvider {
    /// Returns a Cloudflare API token.
    fn api_token(&self) -> Result<String>;

    /// Returns the target zone ID.
    fn zone_id(&self) -> Result<String>;

    /// Helper for implementors that want consistent required-field handling.
    fn require_value(name: &str, value: &str) -> Result<String>
    where
        Self: Sized,
    {
        if value.trim().is_empty() {
            Err(AuthError::MissingValue(name.to_string()))
        } else {
            Ok(value.to_string())
        }
    }
}
