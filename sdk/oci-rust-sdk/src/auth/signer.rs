use super::{AuthError, ConfigurationProvider, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::pkcs1v15::{Signature, SigningKey};
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

        // Parse PEM - only decode the first PEM block, ignore any trailing content
        // This matches Oracle Go SDK behavior which uses pem.Decode()
        let pem_data = pem::parse(&private_key_pem)
            .map_err(|e| AuthError::CryptoError(format!("Failed to parse PEM: {}", e)))?;

        // Try PKCS#1 format first (BEGIN RSA PRIVATE KEY), then PKCS#8 (BEGIN PRIVATE KEY)
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

        // Build signing string - MUST match Go SDK format exactly
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

            // Add content-length first, then content-type, then x-content-sha256
            // This matches Go SDK's defaultBodyHeaders order
            signing_string.push_str(&format!("\ncontent-length: {}", content_length));
            signing_headers.push("content-length".to_string());

            // Add content-type if present in headers
            for (key, value) in headers {
                let key_lower = key.to_lowercase();
                if key_lower == "content-type" {
                    signing_string.push_str(&format!("\ncontent-type: {}", value));
                    signing_headers.push("content-type".to_string());
                    break;
                }
            }

            signing_string.push_str(&format!("\nx-content-sha256: {}", content_sha256));
            signing_headers.push("x-content-sha256".to_string());
        }

        // Sign the string using PKCS#1 v1.5 (NOT PSS!) to match Go SDK
        // Go SDK uses: rsa.SignPKCS1v15(rand.Reader, privateKey, crypto.SHA256, hashed)
        
        // First, hash the signing string with SHA256
        let mut hasher = Sha256::new();
        hasher.update(signing_string.as_bytes());
        let digest = hasher.finalize();
        
        // Debug: print signing string
        eprintln!("=== DEBUG: Signing String ===");
        eprintln!("{}", signing_string);
        eprintln!("=== END Signing String ===\n");
        
        // Sign using PKCS#1 v1.5 with SHA256 DigestInfo
        // Go SDK uses: rsa.SignPKCS1v15(rand.Reader, privateKey, crypto.SHA256, hashed)
        // This adds the DigestInfo prefix for SHA256 before signing
        
        // DigestInfo for SHA256: 0x3031300d060960864801650304020105000420 + hash
        let mut digest_info = vec![
            0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86,
            0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05,
            0x00, 0x04, 0x20,
        ];
        digest_info.extend_from_slice(&digest);
        
        let signing_key = SigningKey::<Sha256>::new_unprefixed(self.private_key.clone());
        let signature: Signature = signing_key.sign(&digest_info);
        let signature_b64 = BASE64.encode(signature.to_bytes().as_ref());
        
        // Debug: print authorization header
        eprintln!("=== DEBUG: Authorization Header ===");
        eprintln!("Headers: {}", signing_headers.join(" "));
        eprintln!("Signature: {}", signature_b64);
        eprintln!("=== END Authorization ===\n");

        // Build Authorization header - exact format from Go SDK
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
