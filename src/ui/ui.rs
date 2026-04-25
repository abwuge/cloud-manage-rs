use dialoguer::{theme::ColorfulTheme, Select};

pub const MENU_OPTIONS: &[&str] = &[
    "Create Instance",
    "Snipe Instance (retry until success)",
    "Reconfigure",
    "Quick Config (Instance Only)",
    "View Current Config",
    "Exit",
];

pub fn print_banner() {
    println!("\n╔════════════════════════════════════════╗");
    println!("║  Oracle Cloud Instance Manager         ║");
    println!("╚════════════════════════════════════════╝\n");
}

pub fn pause_for_user() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("\nPress Enter to continue...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(())
}

pub fn show_menu() -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select operation")
        .items(MENU_OPTIONS)
        .default(0)
        .interact()
        .map_err(|e| e.into())
}
