use oci_rust_sdk::auth::{ConfigurationProvider, FileConfigProvider};
use oci_rust_sdk::compute::ComputeClient;
use std::path::Path;

/// Example: Test OCI API connection and authentication
/// 
/// Environment variables:
/// - OCI_CONFIG_FILE: Path to OCI config file (default: ~/.oci/config)
/// - OCI_CONFIG_PROFILE: Profile name to use (default: DEFAULT)
/// - OCI_TEST_INSTANCE_ID: Instance ID to test (optional, uses fake ID if not set)
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OCI SDK Connection Test ===\n");
    
    // Get config file path from environment or use default
    let config_path = std::env::var("OCI_CONFIG_FILE")
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .expect("Cannot determine home directory");
            format!("{}/.oci/config", home)
        });
    
    let profile = std::env::var("OCI_CONFIG_PROFILE").unwrap_or_else(|_| "DEFAULT".to_string());
    
    println!("Loading configuration...");
    println!("  Config file: {}", config_path);
    println!("  Profile: {}", profile);
    
    let config = FileConfigProvider::from_file(Path::new(&config_path), &profile)?;
    
    println!("✓ Configuration loaded successfully");
    println!("  Region: {}", config.region()?);
    println!("  User: {}", config.user_id()?);
    println!("  Tenancy: {}", config.tenancy_id()?);
    println!("  Fingerprint: {}", config.fingerprint()?);
    
    // Create compute client
    println!("\nInitializing Compute client...");
    let compute_client = ComputeClient::new(&config)?;
    println!("✓ Compute client initialized");
    
    // Test API call - use instance ID from environment if provided
    println!("\nTesting API authentication...");
    let test_instance_id = std::env::var("OCI_TEST_INSTANCE_ID")
        .unwrap_or_else(|_| format!("ocid1.instance.oc1.{}.test", config.region().unwrap_or_default()));
    
    println!("  Testing with instance ID: {}", test_instance_id);
    
    match compute_client.get_instance(&test_instance_id).await {
        Ok(instance) => {
            println!("✓ API call successful!");
            println!("  Instance: {}", instance.display_name.unwrap_or_else(|| "N/A".to_string()));
            println!("  State: {:?}", instance.lifecycle_state);
        }
        Err(e) => {
            let error_msg = format!("{:?}", e);
            if error_msg.contains("404") || error_msg.contains("NotAuthorizedOrNotFound") {
                println!("✓ Authentication successful!");
                println!("  Your OCI credentials are working correctly.");
            } else if error_msg.contains("401") || error_msg.contains("NotAuthenticated") {
                println!("✗ Authentication failed!");
                println!("  Error: {}", error_msg);
                println!("\n  Please check:");
                println!("  1. The public key is uploaded to OCI console");
                println!("  2. The fingerprint matches");
                println!("  3. The user OCID is correct");
                return Err("Authentication failed".into());
            } else {
                println!("✗ API call failed:");
                println!("  {}", error_msg);
                return Err(e);
            }
        }
    }
    
    println!("\n=== Test Complete ===");
    
    Ok(())
}
