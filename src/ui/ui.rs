use dialoguer::{Select, theme::ColorfulTheme};

pub const MAIN_MENU_OPTIONS: &[&str] = &["Oracle Cloud", "Cloudflare", "Exit"];

pub const OCI_MENU_OPTIONS: &[&str] = &[
    "Create Instance",
    "Snipe Instance (retry until success)",
    "Quick Config (Instance Only)",
    "Configure Oracle Cloud",
    "View Oracle Config",
    "Back",
];

pub const CLOUDFLARE_MENU_OPTIONS: &[&str] = &[
    "List DNS records",
    "Upsert DNS record",
    "Delete DNS record",
    "Configure Cloudflare",
    "View Cloudflare Config",
    "Back",
];

pub fn print_banner() {
    println!("\n╔════════════════════════════════════════╗");
    println!("║  Cloud Manage                          ║");
    println!("╚════════════════════════════════════════╝\n");
}

pub fn pause_for_user() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("\nPress Enter to continue...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(())
}

pub fn show_main_menu() -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select provider")
        .items(MAIN_MENU_OPTIONS)
        .default(0)
        .interact()
        .map_err(|e| e.into())
}

pub fn show_oci_menu() -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Oracle Cloud")
        .items(OCI_MENU_OPTIONS)
        .default(0)
        .interact()
        .map_err(|e| e.into())
}

pub fn show_cloudflare_menu() -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Cloudflare")
        .items(CLOUDFLARE_MENU_OPTIONS)
        .default(0)
        .interact()
        .map_err(|e| e.into())
}
