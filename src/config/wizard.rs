use crate::config::{
    CloudflareConfig, InstanceConfigFile, InstanceSettings, NetworkSettings, OracleConfig,
};
use crate::ui::ghost_input::ghost_input;
use dialoguer::{Confirm, Select, theme::ColorfulTheme};
use oci_rust_sdk::compute::ComputeClient;
use oci_rust_sdk::compute::models::{AvailabilityDomain, Image, Subnet};
use std::sync::Arc;

pub struct ConfigWizard {
    theme: ColorfulTheme,
}

impl ConfigWizard {
    pub fn new() -> Self {
        Self {
            theme: ColorfulTheme::default(),
        }
    }

    pub async fn run(
        &self,
        default_config: &InstanceConfigFile,
    ) -> Result<InstanceConfigFile, Box<dyn std::error::Error + Send + Sync>> {
        println!("\n🚀 Oracle Cloud Instance Configuration Wizard");
        println!("================================\n");

        println!("🔄 Connecting to Oracle Cloud API...");
        let client = Arc::new(ComputeClient::new(&default_config.oci)?);
        println!("✅ Connected\n");

        // Most runs accept the default compartment (== tenancy), so prefetch with
        // that candidate now and discard later if the user picks something else.
        let candidate_compartment = self.candidate_compartment(default_config);

        let image_fetch = {
            let client = client.clone();
            let amd_id = default_config.oracle.image_id_amd.clone();
            let arm_id = default_config.oracle.image_id_arm.clone();
            tokio::spawn(async move {
                let amd = if is_valid_ocid(&amd_id, "image") {
                    client
                        .get_image(&amd_id)
                        .await
                        .map_err(|e| e.to_string())
                        .ok()
                } else {
                    None
                };
                let arm = if is_valid_ocid(&arm_id, "image") {
                    client
                        .get_image(&arm_id)
                        .await
                        .map_err(|e| e.to_string())
                        .ok()
                } else {
                    None
                };
                (amd, arm)
            })
        };
        let mut ad_fetch = Some({
            let client = client.clone();
            let cid = candidate_compartment.clone();
            tokio::spawn(async move {
                client
                    .list_availability_domains(&cid)
                    .await
                    .map_err(|e| e.to_string())
            })
        });
        let mut subnet_fetch = Some({
            let client = client.clone();
            let cid = candidate_compartment.clone();
            tokio::spawn(async move { client.list_subnets(&cid).await.map_err(|e| e.to_string()) })
        });

        println!("📋 Basic Information");
        println!("--------------------------------");

        let display_name = ghost_input("Instance Name", "", &default_config.instance.display_name)?;

        let compartment_id = ghost_input(
            "Compartment OCID (usually tenancy root)",
            "",
            &candidate_compartment,
        )?;

        if compartment_id != candidate_compartment {
            ad_fetch = None;
            subnet_fetch = None;
        }

        println!("\n📍 Resolving availability domains...");
        let ad_result: Vec<AvailabilityDomain> = match ad_fetch {
            Some(handle) => handle
                .await
                .map_err(|e| format!("AD fetch task failed: {e}"))?
                .unwrap_or_else(|e| {
                    println!("⚠️  Failed to fetch availability domains: {}", e);
                    Vec::new()
                }),
            None => client
                .list_availability_domains(&compartment_id)
                .await
                .unwrap_or_else(|e| {
                    println!("⚠️  Failed to fetch availability domains: {}", e);
                    Vec::new()
                }),
        };
        let availability_domain =
            self.pick_availability_domain(&ad_result, &default_config.oracle.availability_domain)?;

        println!("\n💿 Image Configuration");
        let (amd_image, arm_image) = image_fetch
            .await
            .map_err(|e| format!("image fetch task failed: {e}"))?;
        let (image_id_amd, image_id_arm) = match (amd_image, arm_image) {
            (Some(amd), Some(arm)) => {
                println!(
                    "  AMD: {}",
                    amd.display_name.as_deref().unwrap_or("Unknown")
                );
                println!(
                    "  ARM: {}",
                    arm.display_name.as_deref().unwrap_or("Unknown")
                );
                (amd.id, arm.id)
            }
            _ => {
                self.select_images(&client, &compartment_id, default_config)
                    .await?
            }
        };

        println!("\n🌐 Network Configuration");
        println!("------------------------");

        println!("Resolving subnets...");
        let subnets: Vec<Subnet> = match subnet_fetch {
            Some(handle) => handle
                .await
                .map_err(|e| format!("subnet fetch task failed: {e}"))?
                .unwrap_or_else(|e| {
                    println!("⚠️  Failed to fetch subnets: {}", e);
                    Vec::new()
                }),
            None => client
                .list_subnets(&compartment_id)
                .await
                .unwrap_or_else(|e| {
                    println!("⚠️  Failed to fetch subnets: {}", e);
                    Vec::new()
                }),
        };
        let (subnet_id, subnet_cidr, subnet_ipv6_cidr) =
            self.pick_subnet(&subnets, &default_config.oracle.subnet_id)?;

        let use_private_ip = Confirm::with_theme(&self.theme)
            .with_prompt("Specify private IPv4 address?")
            .default(default_config.network.private_ip.is_some())
            .interact()?;

        let private_ip = if use_private_ip {
            let prefix = subnet_cidr
                .as_deref()
                .and_then(extract_ipv4_prefix)
                .unwrap_or_default();
            let label = match subnet_cidr.as_deref() {
                Some(c) => format!("Private IPv4 address (Subnet: {})", c),
                None => "Private IPv4 address".to_string(),
            };
            let default_full = default_config.network.private_ip.as_deref().unwrap_or("");
            let default_suffix = if prefix.is_empty() {
                default_full
            } else {
                default_full.strip_prefix(&prefix).unwrap_or(default_full)
            };
            Some(ghost_input(&label, &prefix, default_suffix)?)
        } else {
            None
        };

        let assign_public_ip = Confirm::with_theme(&self.theme)
            .with_prompt("Assign public IPv4?")
            .default(default_config.network.assign_public_ip)
            .interact()?;

        let assign_ipv6 = Confirm::with_theme(&self.theme)
            .with_prompt("Enable IPv6?")
            .default(default_config.network.assign_ipv6)
            .interact()?;

        let ipv6_address = if assign_ipv6 {
            match subnet_ipv6_cidr.as_deref() {
                Some(ipv6_cidr) => {
                    let use_ipv6_address = Confirm::with_theme(&self.theme)
                        .with_prompt("Specify IPv6 address?")
                        .default(default_config.network.ipv6_address.is_some())
                        .interact()?;
                    if use_ipv6_address {
                        let prefix = extract_ipv6_prefix(ipv6_cidr).unwrap_or_default();
                        let default_full =
                            default_config.network.ipv6_address.as_deref().unwrap_or("");
                        let default_suffix = if prefix.is_empty() {
                            default_full
                        } else {
                            default_full.strip_prefix(&prefix).unwrap_or(default_full)
                        };
                        let label = format!("IPv6 address (Subnet: {})", ipv6_cidr);
                        Some(ghost_input(&label, &prefix, default_suffix)?)
                    } else {
                        None
                    }
                }
                None => {
                    println!("⚠️  Subnet does not have IPv6 enabled");
                    None
                }
            }
        } else {
            None
        };

        let ssh_public_key =
            ghost_input("SSH public key", "", &default_config.oracle.ssh_public_key)?;

        let use_hostname = Confirm::with_theme(&self.theme)
            .with_prompt("Set hostname label?")
            .default(default_config.network.hostname_label.is_some())
            .interact()?;

        let hostname_label = if use_hostname {
            let existing = default_config
                .network
                .hostname_label
                .as_deref()
                .unwrap_or("");
            Some(ghost_input("Hostname label", "", existing)?)
        } else {
            None
        };

        println!("\n⚙️  Instance Type (Shape)");
        let instance_types = vec![
            "AMD Micro Instance (VM.Standard.E2.1.Micro)",
            "ARM Flex Instance (VM.Standard.A1.Flex)",
        ];
        let default_type_index = if default_config.instance.instance_type == "amd" {
            0
        } else {
            1
        };
        let instance_type_index = Select::with_theme(&self.theme)
            .with_prompt("Select instance type")
            .items(&instance_types)
            .default(default_type_index)
            .interact()?;
        let instance_type = if instance_type_index == 0 {
            "amd"
        } else {
            "arm"
        }
        .to_string();

        let (arm_ocpus, arm_memory_gb) = if instance_type == "arm" {
            let ocpu_options = vec![
                "1 OCPU (6 GB)",
                "2 OCPU (12 GB)",
                "3 OCPU (18 GB)",
                "4 OCPU (24 GB)",
            ];
            let default_ocpu_index = default_config
                .instance
                .arm_ocpus
                .unwrap_or(2)
                .saturating_sub(1) as usize;
            let ocpu_index = Select::with_theme(&self.theme)
                .with_prompt("Select OCPU and memory configuration (1 OCPU = 6 GB)")
                .items(&ocpu_options)
                .default(default_ocpu_index.min(3))
                .interact()?;
            let ocpus = (ocpu_index + 1) as u8;
            (Some(ocpus), Some(ocpus * 6))
        } else {
            (None, None)
        };

        println!("\n💾 Storage Configuration");
        let boot_volume_default = default_config.instance.boot_volume_size_gb.to_string();
        let boot_volume_size_gb: i64 = loop {
            let raw = ghost_input("Boot volume size (GB)", "", &boot_volume_default)?;
            match raw.parse::<i64>() {
                Ok(v) => break v,
                Err(_) => println!("⚠️  Please enter a valid integer"),
            }
        };

        let config = InstanceConfigFile {
            oci: default_config.oci.clone(),
            cloudflare: default_config.cloudflare.clone(),
            oracle: OracleConfig {
                compartment_id,
                availability_domain,
                subnet_id,
                image_id_amd,
                image_id_arm,
                ssh_public_key,
            },
            instance: InstanceSettings {
                instance_type,
                display_name,
                arm_ocpus,
                arm_memory_gb,
                boot_volume_size_gb,
            },
            network: NetworkSettings {
                assign_public_ip,
                assign_ipv6,
                private_ip,
                ipv6_address,
                hostname_label,
            },
            snipe: default_config.snipe.clone(),
        };

        println!("\n📝 Configuration Summary");
        println!("============");
        println!(
            "Instance Type: {}",
            if config.instance.instance_type == "amd" {
                "AMD Micro"
            } else {
                "ARM Flex"
            }
        );
        println!("Instance Name: {}", config.instance.display_name);
        if let (Some(ocpus), Some(memory)) =
            (config.instance.arm_ocpus, config.instance.arm_memory_gb)
        {
            println!("OCPU: {}, Memory: {} GB", ocpus, memory);
        }
        println!("Boot Volume: {} GB", config.instance.boot_volume_size_gb);
        println!(
            "Public IPv4: {}",
            if config.network.assign_public_ip {
                "Yes"
            } else {
                "No"
            }
        );
        println!(
            "IPv6: {}",
            if config.network.assign_ipv6 {
                "Yes"
            } else {
                "No"
            }
        );
        if let Some(ip) = &config.network.private_ip {
            println!("Private IP: {}", ip);
        }
        if let Some(ipv6) = &config.network.ipv6_address {
            println!("IPv6 Address: {}", ipv6);
        }
        if let Some(hostname) = &config.network.hostname_label {
            println!("Hostname: {}", hostname);
        }

        Ok(config)
    }

    /// Resolution order: valid config value -> tenancy from OCI auth section -> raw default.
    fn candidate_compartment(&self, default_config: &InstanceConfigFile) -> String {
        if is_valid_ocid(&default_config.oracle.compartment_id, "compartment")
            || is_valid_ocid(&default_config.oracle.compartment_id, "tenancy")
        {
            return default_config.oracle.compartment_id.clone();
        }
        if !default_config.oci.tenancy.is_empty() {
            return default_config.oci.tenancy.clone();
        }
        default_config.oracle.compartment_id.clone()
    }

    fn pick_availability_domain(
        &self,
        domains: &[AvailabilityDomain],
        default_ad: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if domains.is_empty() {
            return Ok(ghost_input("Availability Domain", "", default_ad)?);
        }

        let names: Vec<String> = domains.iter().map(|d| d.name.clone()).collect();
        let default_index = names.iter().position(|n| n == default_ad).unwrap_or(0);
        let selection = Select::with_theme(&self.theme)
            .with_prompt("Select Availability Domain")
            .items(&names)
            .default(default_index)
            .interact()?;
        Ok(names[selection].clone())
    }

    fn pick_subnet(
        &self,
        subnets: &[Subnet],
        default_subnet: &str,
    ) -> Result<(String, Option<String>, Option<String>), Box<dyn std::error::Error + Send + Sync>>
    {
        if subnets.is_empty() {
            let subnet_id = ghost_input("Subnet OCID", "", default_subnet)?;
            let cidr = ghost_input("Subnet CIDR (leave blank if unknown)", "", "")?;
            let cidr = if cidr.trim().is_empty() {
                None
            } else {
                Some(cidr)
            };
            return Ok((subnet_id, cidr, None));
        }

        let items: Vec<String> = subnets
            .iter()
            .map(|s| {
                format!(
                    "{} ({})",
                    s.display_name.as_deref().unwrap_or("Unnamed"),
                    s.cidr_block.as_deref().unwrap_or("N/A"),
                )
            })
            .collect();
        let default_index = subnets
            .iter()
            .position(|s| s.id == default_subnet)
            .unwrap_or(0);
        let selection = Select::with_theme(&self.theme)
            .with_prompt("Select subnet")
            .items(&items)
            .default(default_index)
            .interact()?;

        let s = &subnets[selection];
        Ok((
            s.id.clone(),
            s.cidr_block.clone(),
            s.ipv6_cidr_block.clone(),
        ))
    }

    async fn select_images(
        &self,
        client: &ComputeClient,
        compartment_id: &str,
        default_config: &InstanceConfigFile,
    ) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        use std::collections::BTreeMap;

        'distribution_select: loop {
            let distributions: Vec<(&str, &str)> = vec![
                ("Oracle Linux", "Oracle Linux"),
                ("Ubuntu", "Canonical Ubuntu"),
                ("Windows Server", "Windows"),
                ("CentOS", "CentOS"),
                ("Red Hat Enterprise Linux", "Red Hat Enterprise Linux"),
            ];

            let mut dist_items: Vec<String> =
                distributions.iter().map(|(d, _)| d.to_string()).collect();
            dist_items.push("← Back".to_string());

            let dist_selection = Select::with_theme(&self.theme)
                .with_prompt("Select OS distribution")
                .items(&dist_items)
                .default(0)
                .interact()?;
            if dist_selection == dist_items.len() - 1 {
                return Err("User cancelled selection".into());
            }
            let (_, api_name) = distributions[dist_selection];

            let images = match client
                .list_images_filtered(compartment_id, Some(api_name), None)
                .await
            {
                Ok(imgs) => imgs,
                Err(e) => {
                    println!("⚠️  Failed to fetch {} images: {}", api_name, e);
                    continue 'distribution_select;
                }
            };
            if images.is_empty() {
                println!("⚠️  No {} images found", api_name);
                continue 'distribution_select;
            }

            let mut version_map: BTreeMap<String, Vec<&Image>> = BTreeMap::new();
            for img in &images {
                let display_name = img.display_name.clone().unwrap_or_default();
                let name_lower = display_name.to_lowercase();
                if name_lower.contains("minimal")
                    || name_lower.contains("gpu")
                    || name_lower.contains("stig")
                    || name_lower.contains("core")
                {
                    continue;
                }
                let version = img
                    .operating_system_version
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string());
                let major_version = if api_name == "Windows" {
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
                version_map
                    .entry(major_version)
                    .or_insert_with(Vec::new)
                    .push(img);
            }

            if version_map.is_empty() {
                println!("⚠️  No available images found");
                continue 'distribution_select;
            }

            let mut versions: Vec<_> = version_map.keys().collect();
            versions.sort_by(|a, b| b.cmp(a));

            let mut version_items: Vec<String> = versions
                .iter()
                .take(3)
                .map(|v| format!("Version {}", v))
                .collect();
            version_items.push("← Back".to_string());
            let version_selection = Select::with_theme(&self.theme)
                .with_prompt("Select version")
                .items(&version_items)
                .default(0)
                .interact()?;
            if version_selection == version_items.len() - 1 {
                continue 'distribution_select;
            }
            let selected_version = versions[version_selection];
            let images_in_version = &version_map[selected_version];

            let arm_images: Vec<_> = images_in_version
                .iter()
                .filter(|img| {
                    img.display_name
                        .as_deref()
                        .map(|n| {
                            let l = n.to_lowercase();
                            l.contains("aarch64") || l.contains("arm")
                        })
                        .unwrap_or(false)
                })
                .collect();
            let amd_images: Vec<_> = images_in_version
                .iter()
                .filter(|img| {
                    img.display_name
                        .as_deref()
                        .map(|n| {
                            let l = n.to_lowercase();
                            !l.contains("aarch64") && !l.contains("arm")
                        })
                        .unwrap_or(false)
                })
                .collect();

            let image_id_amd = if !amd_images.is_empty() {
                let latest = amd_images
                    .iter()
                    .max_by_key(|img| &img.display_name)
                    .unwrap();
                println!(
                    "\n✅ AMD Image: {}",
                    latest.display_name.as_deref().unwrap_or("N/A")
                );
                latest.id.clone()
            } else {
                println!("⚠️  No AMD images found");
                ghost_input("AMD Image OCID", "", &default_config.oracle.image_id_amd)?
            };
            let image_id_arm = if !arm_images.is_empty() {
                let latest = arm_images
                    .iter()
                    .max_by_key(|img| &img.display_name)
                    .unwrap();
                println!(
                    "✅ ARM Image: {}",
                    latest.display_name.as_deref().unwrap_or("N/A")
                );
                latest.id.clone()
            } else {
                println!("⚠️  No ARM images found");
                ghost_input("ARM Image OCID", "", &default_config.oracle.image_id_arm)?
            };

            return Ok((image_id_amd, image_id_arm));
        }
    }

    pub async fn quick_config(
        &self,
        base_config: &InstanceConfigFile,
    ) -> Result<InstanceConfigFile, Box<dyn std::error::Error + Send + Sync>> {
        println!("\n⚡ Quick Configuration Mode");
        println!("================\n");

        let instance_types = vec!["AMD Micro Instance", "ARM Flex Instance"];
        let instance_type_index = Select::with_theme(&self.theme)
            .with_prompt("Select instance type")
            .items(&instance_types)
            .default(0)
            .interact()?;
        let instance_type = if instance_type_index == 0 {
            "amd"
        } else {
            "arm"
        }
        .to_string();

        let display_name = ghost_input("Instance Name", "", &base_config.instance.display_name)?;

        let (arm_ocpus, arm_memory_gb) = if instance_type == "arm" {
            let ocpu_options = vec![
                "1 OCPU (6 GB)",
                "2 OCPU (12 GB)",
                "3 OCPU (18 GB)",
                "4 OCPU (24 GB)",
            ];
            let ocpu_index = Select::with_theme(&self.theme)
                .with_prompt("Select configuration")
                .items(&ocpu_options)
                .default(1)
                .interact()?;
            let ocpus = (ocpu_index + 1) as u8;
            (Some(ocpus), Some(ocpus * 6))
        } else {
            (None, None)
        };

        let assign_public_ip = Confirm::with_theme(&self.theme)
            .with_prompt("Assign public IPv4?")
            .default(true)
            .interact()?;

        let mut config = base_config.clone();
        config.instance.instance_type = instance_type;
        config.instance.display_name = display_name;
        config.instance.arm_ocpus = arm_ocpus;
        config.instance.arm_memory_gb = arm_memory_gb;
        config.network.assign_public_ip = assign_public_ip;
        Ok(config)
    }

    pub async fn cloudflare_config(
        &self,
        base_config: &InstanceConfigFile,
    ) -> Result<InstanceConfigFile, Box<dyn std::error::Error + Send + Sync>> {
        println!("\n☁️  Cloudflare DNS Configuration");
        println!("===============================\n");
        println!("Use an API token scoped to the target zone with Zone:DNS Edit permission.");
        println!(
            "Domain accepts either a zone like `example.com` or a record mapping like `www@example.com`."
        );

        let api_token = ghost_input(
            "API token",
            "",
            &mask_existing_secret(&base_config.cloudflare.api_token),
        )?;
        let api_token = if api_token.chars().all(|c| c == '*') {
            base_config.cloudflare.api_token.clone()
        } else {
            api_token
        };
        let default_domain = default_cloudflare_domain(&base_config.cloudflare);
        let domain = ghost_input("Domain", "", &default_domain)?;
        let (record_name, zone_name) = parse_cloudflare_domain(&domain);

        let mut config = base_config.clone();
        config.cloudflare = CloudflareConfig {
            api_token,
            zone_name,
            record_name,
        };
        Ok(config)
    }
}

fn default_cloudflare_domain(config: &CloudflareConfig) -> String {
    match (&config.record_name, config.zone_name.is_empty()) {
        (Some(record), false) => format!("{}@{}", record, config.zone_name),
        _ => config.zone_name.clone(),
    }
}

fn parse_cloudflare_domain(value: &str) -> (Option<String>, String) {
    let trimmed = value.trim();
    if let Some((record, zone)) = trimmed.split_once('@') {
        let record = record.trim();
        let zone = zone.trim();
        let record = if record.is_empty() {
            None
        } else {
            Some(record.to_string())
        };
        (record, zone.to_string())
    } else {
        (None, trimmed.to_string())
    }
}

fn mask_existing_secret(value: &str) -> String {
    if value.is_empty() {
        String::new()
    } else {
        "********".to_string()
    }
}

fn is_valid_ocid(s: &str, kind: &str) -> bool {
    let prefix = format!("ocid1.{}.", kind);
    s.starts_with(&prefix) && !s.contains("your-") && s.len() > prefix.len() + 10
}

/// IPv4 subnet CIDR -> first-two-octets prefix ending with '.'.
/// Returns `None` when the CIDR cannot be parsed (no fallback is invented).
fn extract_ipv4_prefix(cidr: &str) -> Option<String> {
    let ip_part = cidr.split('/').next()?;
    let parts: Vec<&str> = ip_part.split('.').collect();
    if parts.len() >= 3 {
        Some(format!("{}.{}.", parts[0], parts[1]))
    } else {
        None
    }
}

/// IPv6 subnet CIDR -> address prefix ending in a single ':'.
/// Example: `2001:db8:1234:5678::/64` -> `2001:db8:1234:5678:`.
/// Returns `None` when the CIDR cannot be parsed.
fn extract_ipv6_prefix(cidr: &str) -> Option<String> {
    let ip_part = cidr.split('/').next()?;
    let trimmed = ip_part.trim_end_matches(':');
    if trimmed.is_empty() {
        None
    } else {
        Some(format!("{}:", trimmed))
    }
}
