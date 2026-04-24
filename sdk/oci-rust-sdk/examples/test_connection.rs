use oci_rust_sdk::auth::{ConfigurationProvider, FileConfigProvider};
use oci_rust_sdk::compute::ComputeClient;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OCI SDK Connection Test ===\n");
    
    // Load configuration from config/oci_config
    println!("Loading configuration from ./config/oci_config...");
    let config = FileConfigProvider::from_file(Path::new("./config/oci_config"), "DEFAULT")?;
    
    println!("✓ Configuration loaded successfully");
    println!("  Region: {}", config.region()?);
    println!("  User: {}", config.user_id()?);
    println!("  Tenancy: {}", config.tenancy_id()?);
    println!("  Fingerprint: {}", config.fingerprint()?);
    
    // Create compute client
    println!("\nInitializing Compute client...");
    let compute_client = ComputeClient::new(&config)?;
    println!("✓ Compute client initialized");
    
    // Test API call - try to get a non-existent instance (this will test authentication)
    println!("\nTesting API authentication...");
    let test_instance_id = "ocid1.instance.oc1.ap-singapore-1.test";
    match compute_client.get_instance(test_instance_id).await {
        Ok(_) => {
            println!("✓ API call successful (unexpected - test instance exists?)");
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("404") || error_msg.contains("NotAuthorizedOrNotFound") {
                println!("✓ Authentication successful! (404 error is expected for test instance)");
                println!("  Your OCI credentials are working correctly.");
            } else if error_msg.contains("401") || error_msg.contains("NotAuthenticated") {
                println!("✗ Authentication failed!");
                println!("  Error: {}", error_msg);
                println!("\n  Please check:");
                println!("  1. The public key is uploaded to OCI console");
                println!("  2. The fingerprint matches");
                println!("  3. The user OCID is correct");
            } else {
                println!("✗ API call failed with unexpected error:");
                println!("  {}", error_msg);
            }
        }
    }
    
    println!("\n=== Test Complete ===");
    
    Ok(())
}
