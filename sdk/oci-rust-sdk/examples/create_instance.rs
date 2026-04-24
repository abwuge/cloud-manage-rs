use oci_rust_sdk::auth::{ConfigurationProvider, FileConfigProvider};
use oci_rust_sdk::compute::{
    ComputeClient, CreateVnicDetails, InstanceSourceDetails, LaunchInstanceDetails,
};
use std::env;
use std::path::Path;

/// Example: List available resources and create compute instance
/// 
/// Environment variables:
/// - OCI_CONFIG_FILE: Path to OCI config file (default: ~/.oci/config)
/// - OCI_CONFIG_PROFILE: Profile name to use (default: DEFAULT)
/// 
/// Optional variables for instance creation:
/// - OCI_AVAILABILITY_DOMAIN: Availability domain name
/// - OCI_IMAGE_ID: Image ID to use
/// - OCI_SHAPE: Instance shape (e.g., VM.Standard.E2.1.Micro)
/// - OCI_SUBNET_ID: Subnet ID for networking
/// - OCI_INSTANCE_NAME: Name for the new instance
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment variables
    let config_file = env::var("OCI_CONFIG_FILE")
        .unwrap_or_else(|_| format!("{}/.oci/config", env::var("HOME").unwrap()));
    let profile = env::var("OCI_CONFIG_PROFILE").unwrap_or_else(|_| "DEFAULT".to_string());

    println!("Loading config file: {}", config_file);
    println!("Using profile: {}", profile);

    // Load configuration
    let config = FileConfigProvider::from_file(Path::new(&config_file), &profile)?;
    let region = config.region()?;
    let compartment_id = config.tenancy_id()?;

    println!("Region: {}", region);
    println!("Tenancy ID: {}", compartment_id);

    // Create client
    let client = ComputeClient::new(&config)?;

    // 1. List availability domains
    println!("\n=== Availability Domains ===");
    let domains = client.list_availability_domains(&compartment_id).await?;
    for (i, domain) in domains.iter().enumerate() {
        println!("{}. {}", i + 1, domain.name);
    }

    if domains.is_empty() {
        println!("No availability domains found");
        return Ok(());
    }

    // 2. List images (first 10)
    println!("\n=== Available Images (first 10) ===");
    let images = client.list_images(&compartment_id).await?;
    for (i, image) in images.iter().take(10).enumerate() {
        println!(
            "{}. {} - {} {}",
            i + 1,
            image.display_name.as_deref().unwrap_or("N/A"),
            image.operating_system.as_deref().unwrap_or("N/A"),
            image.operating_system_version.as_deref().unwrap_or("")
        );
        println!("   ID: {}", image.id);
    }

    if images.is_empty() {
        println!("No images found");
        return Ok(());
    }

    // 3. List shapes (first 10)
    println!("\n=== Available Shapes (first 10) ===");
    let shapes = client.list_shapes(&compartment_id).await?;
    for (i, shape) in shapes.iter().take(10).enumerate() {
        println!(
            "{}. {} - {} OCPUs, {} GB memory",
            i + 1,
            shape.shape,
            shape.ocpus.unwrap_or(0.0),
            shape.memory_in_gbs.unwrap_or(0.0)
        );
    }

    if shapes.is_empty() {
        println!("No shapes found");
        return Ok(());
    }

    // 4. List VCNs
    println!("\n=== Virtual Cloud Networks ===");
    let vcns = client.list_vcns(&compartment_id).await?;
    for (i, vcn) in vcns.iter().enumerate() {
        println!(
            "{}. {} ({})",
            i + 1,
            vcn.display_name.as_deref().unwrap_or("N/A"),
            vcn.cidr_block.as_deref().unwrap_or("N/A")
        );
        println!("   ID: {}", vcn.id);
    }

    if vcns.is_empty() {
        println!("No VCNs found");
        return Ok(());
    }

    // 5. List subnets
    println!("\n=== Subnets ===");
    let subnets = client.list_subnets(&compartment_id).await?;
    for (i, subnet) in subnets.iter().enumerate() {
        println!(
            "{}. {} ({})",
            i + 1,
            subnet.display_name.as_deref().unwrap_or("N/A"),
            subnet.cidr_block.as_deref().unwrap_or("N/A")
        );
        println!("   ID: {}", subnet.id);
        println!(
            "   Availability Domain: {}",
            subnet.availability_domain.as_deref().unwrap_or("N/A")
        );
    }

    if subnets.is_empty() {
        println!("No subnets found");
        return Ok(());
    }

    // Example: Create instance (requires user to provide parameters)
    println!("\n=== Create Instance Example ===");
    println!("To create an instance, set the following environment variables:");
    println!("  OCI_AVAILABILITY_DOMAIN - Availability domain name");
    println!("  OCI_IMAGE_ID - Image ID");
    println!("  OCI_SHAPE - Shape name");
    println!("  OCI_SUBNET_ID - Subnet ID");
    println!("  OCI_INSTANCE_NAME - Instance name");

    // Check if all required environment variables are provided
    if let (Ok(ad), Ok(image_id), Ok(shape), Ok(subnet_id), Ok(instance_name)) = (
        env::var("OCI_AVAILABILITY_DOMAIN"),
        env::var("OCI_IMAGE_ID"),
        env::var("OCI_SHAPE"),
        env::var("OCI_SUBNET_ID"),
        env::var("OCI_INSTANCE_NAME"),
    ) {
        println!("\nCreating instance...");

        let launch_details = LaunchInstanceDetails {
            availability_domain: ad,
            compartment_id: compartment_id.clone(),
            shape: shape.clone(),
            display_name: Some(instance_name.clone()),
            hostname_label: Some(instance_name.clone().replace("_", "-")),
            source_details: InstanceSourceDetails::Image {
                image_id: image_id.clone(),
                boot_volume_size_in_gbs: None,
            },
            create_vnic_details: Some(CreateVnicDetails {
                subnet_id: subnet_id.clone(),
                assign_public_ip: Some(true),
                display_name: Some(format!("{}-vnic", instance_name)),
                hostname_label: None,
                private_ip: None,
            }),
            metadata: None,
            shape_config: None,
            freeform_tags: None,
        };

        match client.launch_instance(&launch_details).await {
            Ok(instance) => {
                println!("✓ Instance created successfully!");
                println!("  Instance ID: {}", instance.id);
                println!("  State: {:?}", instance.lifecycle_state);
            }
            Err(e) => {
                println!("✗ Failed to create instance: {}", e);
            }
        }
    } else {
        println!("\nNo creation parameters provided, skipping instance creation.");
    }

    Ok(())
}
