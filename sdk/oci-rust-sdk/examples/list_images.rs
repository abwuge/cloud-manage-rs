use oci_rust_sdk::auth::{ConfigurationProvider, FileConfigProvider};
use oci_rust_sdk::compute::ComputeClient;
use std::collections::BTreeMap;
use std::env;
use std::path::Path;

/// Example: List OCI images in a hierarchical structure
/// 
/// Demonstrates a human-friendly 3-level image listing:
/// Level 1: OS Distribution (Oracle Linux, Ubuntu, Windows, etc.)
/// Level 2: Major Version (latest 3 versions)
/// Level 3: Architecture (ARM/AMD OCIDs for each version)
/// 
/// Environment variables:
/// - OCI_CONFIG_FILE: Path to OCI config file (default: ~/.oci/config)
/// - OCI_CONFIG_PROFILE: Profile name to use (default: DEFAULT)

#[derive(Debug)]
struct ImageInfo {
    ocid: String,
    display_name: String,
    is_arm: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OCI Image Hierarchical List ===\n");
    
    // Load configuration
    let config_file = env::var("OCI_CONFIG_FILE")
        .unwrap_or_else(|_| format!("{}/.oci/config", env::var("HOME").unwrap()));
    let profile = env::var("OCI_CONFIG_PROFILE").unwrap_or_else(|_| "DEFAULT".to_string());
    
    let config = FileConfigProvider::from_file(Path::new(&config_file), &profile)?;
    let compartment_id = config.tenancy_id()?;
    
    println!("Region: {}", config.region()?);
    println!("Compartment: {}\n", compartment_id);
    
    let client = ComputeClient::new(&config)?;
    
    // Define OS distributions (matching OCI web console order)
    let distributions = vec![
        ("Oracle Linux", "Oracle Linux"),
        ("Ubuntu", "Canonical Ubuntu"),
        ("Red Hat", "Red Hat Enterprise Linux"),
        ("CentOS", "CentOS"),
        ("Windows", "Windows"),
        ("AlmaLinux", "AlmaLinux"),
        ("Rocky Linux", "Rocky Linux"),
    ];
    
    println!("📋 Available Platform Images");
    println!("{}", "=".repeat(100));
    println!();
    
    for (display_name, api_name) in &distributions {
        println!("📦 {}", display_name);
        println!("{}", "-".repeat(100));
        
        // Fetch all images for this distribution
        let images = client.list_images_filtered(
            &compartment_id,
            Some(api_name),
            None
        ).await?;
        
        if images.is_empty() {
            println!("  ⚠️  No official images available\n");
            continue;
        }
        
        // Group by major version, filter standard images only
        let mut version_map: BTreeMap<String, Vec<ImageInfo>> = BTreeMap::new();
        
        for img in &images {
            let display_name = img.display_name.clone().unwrap_or_else(|| "N/A".to_string());
            let name_lower = display_name.to_lowercase();
            
            // Skip special variants for Linux (Minimal, GPU, STIG, etc.)
            // Skip Core editions for Windows
            if name_lower.contains("minimal") || 
               name_lower.contains("gpu") || 
               name_lower.contains("stig") ||
               name_lower.contains("core") {
                continue;
            }
            
            let version = img.operating_system_version.clone().unwrap_or_else(|| "Unknown".to_string());
            
            // For Windows, extract version from name
            let major_version = if api_name == &"Windows" {
                if name_lower.contains("2025") {
                    "2025".to_string()
                } else if name_lower.contains("2022") {
                    "2022".to_string()
                } else if name_lower.contains("2019") {
                    "2019".to_string()
                } else if name_lower.contains("2016") {
                    "2016".to_string()
                } else {
                    version.split('.').next().unwrap_or(&version).to_string()
                }
            } else {
                version.split('.').next().unwrap_or(&version).to_string()
            };
            
            let is_arm = name_lower.contains("aarch64") || name_lower.contains("arm");
            
            let info = ImageInfo {
                ocid: img.id.clone(),
                display_name,
                is_arm,
            };
            
            version_map.entry(major_version).or_insert_with(Vec::new).push(info);
        }
        
        // Show latest 3 major versions
        let mut versions: Vec<_> = version_map.keys().collect();
        versions.sort_by(|a, b| b.cmp(a)); // Descending order
        
        for (idx, major_ver) in versions.iter().take(3).enumerate() {
            let images_in_version = &version_map[*major_ver];
            
            // Find latest ARM and AMD images (by name, which includes date)
            let arm_image = images_in_version.iter()
                .filter(|img| img.is_arm)
                .max_by_key(|img| &img.display_name);
            
            let amd_image = images_in_version.iter()
                .filter(|img| !img.is_arm)
                .max_by_key(|img| &img.display_name);
            
            // Extract version info from the latest image
            let version_info = arm_image.or(amd_image)
                .map(|img| {
                    // Extract version and date from display name
                    let parts: Vec<&str> = img.display_name.split('-').collect();
                    
                    // For Windows, just extract date
                    if api_name == &"Windows" {
                        let date = parts.iter()
                            .find(|p| p.contains('.') && p.len() >= 10)
                            .unwrap_or(&"");
                        return date.to_string();
                    }
                    
                    // For Linux, extract version and date
                    if parts.len() >= 4 {
                        let version = parts.iter()
                            .find(|p| p.chars().next().map_or(false, |c| c.is_numeric()))
                            .unwrap_or(&"");
                        let date = parts.iter()
                            .find(|p| p.contains('.') && p.len() >= 10)
                            .unwrap_or(&"");
                        format!("{} ({})", version, date)
                    } else {
                        String::new()
                    }
                })
                .unwrap_or_default();
            
            if version_info.is_empty() {
                println!("  🔹 Version {}", major_ver);
            } else {
                println!("  🔹 Version {} - Latest: {}", major_ver, version_info);
            }
            
            if let Some(arm) = arm_image {
                println!("     ARM:  {}", arm.ocid);
            } else {
                println!("     ARM:  none");
            }
            
            if let Some(amd) = amd_image {
                println!("     AMD:  {}", amd.ocid);
            } else {
                println!("     AMD:  none");
            }
            
            if idx < versions.len().min(3) - 1 {
                println!();
            }
        }
        
        println!();
    }
    
    Ok(())
}
