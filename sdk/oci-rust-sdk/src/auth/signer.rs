use super::{AuthError, ConfigurationProvider, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::pkcs1v15::SigningKey;
use rsa::signature::{Signer, SignatureEncoding};
use rsa::RsaPrivateKey;
use sha2::{Digest, Sha256};

/// Request signer for OCI API requests
pub struct RequestSigner {
    key_id: String,
    private_key: RsaPrivateKey,
}

impl RequestSigner {
    /// Create a new request signer from a configuration provider
    pub fn new(config: &dyn ConfigurationProvider) -> Result<Self> {
        let key_id = config.key_id()?;
        let private_key_pem = config.private_key()?;

        // Parse PEM and try both PKCS#1 and PKCS#8 formats
        let pem_data = pem::parse(&private_key_pem)
            .map_err(|e| AuthError::CryptoError(format!("Failed to parse PEM: {}", e)))?;

        let private_key = RsaPrivateKey::from_pkcs1_der(&pem_data.contents())
            .or_else(|_| RsaPrivateKey::from_pkcs8_der(&pem_data.contents()))
            .map_err(|e| AuthError::CryptoError(format!("Failed to parse private key: {}", e)))?;

        Ok(Self {
            key_id,
            private_key,
        })
    }

    /// Sign an HTTP request and return the Authorization header value
    pub fn sign_request(
        &self,
        method: &str,
        path: &str,
        host: &str,
        body: Option<&[u8]>,
        headers: &[(&str, &str)],
    ) -> Result<String> {
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();

        let mut signing_headers = vec![
            "(request-target)".to_string(),
            "host".to_string(),
            "date".to_string(),
        ];

        let mut signing_string = format!(
            "(request-target): {} {}\nhost: {}\ndate: {}",
            method.to_lowercase(),
            path,
            host,
            date
        );

        // Add body-related headers if body exists
        if let Some(body_bytes) = body {
            let content_length = body_bytes.len();
            let mut hasher = Sha256::new();
            hasher.update(body_bytes);
            let content_sha256 = BASE64.encode(hasher.finalize());

            signing_string.push_str(&format!("\ncontent-length: {}", content_length));
            signing_headers.push("content-length".to_string());

            for (key, value) in headers {
                if key.to_lowercase() == "content-type" {
                    signing_string.push_str(&format!("\ncontent-type: {}", value));
                    signing_headers.push("content-type".to_string());
                    break;
                }
            }

            signing_string.push_str(&format!("\nx-content-sha256: {}", content_sha256));
            signing_headers.push("x-content-sha256".to_string());
        }

        // Sign using PKCS#1 v1.5 with SHA256
        let signing_key = SigningKey::<Sha256>::new(self.private_key.clone());
        let signature = signing_key.sign(signing_string.as_bytes());
        let signature_b64 = BASE64.encode(signature.to_vec());

        let auth_header = format!(
            r#"Signature version="1",headers="{}",keyId="{}",algorithm="rsa-sha256",signature="{}""#,
            signing_headers.join(" "),
            self.key_id,
            signature_b64
        );

        Ok(auth_header)
    }

    /// Get the date header value for the current request
    pub fn get_date_header() -> String {
        Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string()
    }
}
