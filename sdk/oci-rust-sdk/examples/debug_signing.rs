use oci_rust_sdk::auth::{ConfigurationProvider, FileConfigProvider};
use std::path::Path;
use sha2::{Digest, Sha256};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OCI Signing Debug Tool ===\n");
    
    // Load configuration
    let config = FileConfigProvider::from_file(Path::new("./config/oci_config"), "DEFAULT")?;
    
    println!("Configuration:");
    println!("  User: {}", config.user_id()?);
    println!("  Tenancy: {}", config.tenancy_id()?);
    println!("  Fingerprint: {}", config.fingerprint()?);
    println!("  KeyID: {}\n", config.key_id()?);
    
    // Test signing string
    let method = "get";
    let path = "/20160918/instances/ocid1.instance.oc1.ap-singapore-1.test";
    let host = "iaas.ap-singapore-1.oraclecloud.com";
    let date = "Fri, 24 Apr 2026 11:00:00 GMT";
    
    let signing_string = format!(
        "(request-target): {} {}\nhost: {}\ndate: {}",
        method, path, host, date
    );
    
    println!("Signing String:");
    println!("{}", signing_string);
    println!();
    
    // Hash the signing string
    let mut hasher = Sha256::new();
    hasher.update(signing_string.as_bytes());
    let digest = hasher.finalize();
    
    println!("SHA256 Digest (hex):");
    println!("{:x}", digest);
    println!();
    
    println!("SHA256 Digest (base64):");
    println!("{}", BASE64.encode(&digest));
    println!();
    
    // Show DigestInfo structure
    let mut digest_info = vec![
        0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86,
        0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05,
        0x00, 0x04, 0x20,
    ];
    digest_info.extend_from_slice(&digest);
    
    println!("DigestInfo (hex):");
    for (i, byte) in digest_info.iter().enumerate() {
        print!("{:02x}", byte);
        if (i + 1) % 16 == 0 {
            println!();
        } else {
            print!(" ");
        }
    }
    println!("\n");
    
    println!("DigestInfo length: {} bytes", digest_info.len());
    println!("Expected: 51 bytes (19 prefix + 32 hash)");
    
    Ok(())
}
