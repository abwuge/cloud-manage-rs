# Getting Started

A walkthrough that gets you from a clean checkout to a running cloud
instance. Provider-specific details (auth, resource OCIDs, etc.) live under
[providers/](providers/).

## 1. Prerequisites

- Rust toolchain (`rustup` recommended).
- Credentials for at least one supported cloud provider — currently
  [Oracle Cloud](providers/oracle.md).

## 2. Build

```bash
cargo build --release
# or
cargo run
```

## 3. Provider Auth

Follow the auth setup in your provider's guide:

- [Oracle Cloud Infrastructure](providers/oracle.md#auth-setup)

For OCI specifically, the program expects auth at `./config/oci_config`.

## 4. Run

```bash
cargo run
```

If `./config/instance_config.toml` does not exist (or fails to parse), the
configuration wizard runs automatically.

## 5. The Configuration Wizard

Each prompt has a default. Press Enter to accept it.

For text prompts, the default is shown as **dim ghost text**:

- Start typing to **overwrite** characters from left to right.
- Use **←/→** to move within the current line, **↑/↓** to move across
  wrapped lines, and **Home/End** to jump to the ends.
- **Backspace** / **Delete** edit the buffer in place.
- For private IPv4 / IPv6 addresses, the subnet prefix is locked in front
  and only the host suffix is taken as input.

While the wizard is collecting answers it concurrently fetches resource
lists (availability domains, subnets, image details) in the background, so
selection prompts usually appear instantly.

For the full list of fields and their meaning, see
[Configuration Reference](configuration.md).

## 6. Main Menu

After the wizard you reach the main menu:

| Option | Description |
| --- | --- |
| Create Instance | Launch once using the saved configuration. |
| Snipe Instance | Retry creation in a loop with randomized backoff until it succeeds (or the configured attempt cap is hit). Useful when Always Free capacity is exhausted. |
| Reconfigure | Run the full wizard again. |
| Quick Config (Instance Only) | Edit only instance-level fields (shape, name, OCPUs, public IPv4). |
| View Current Config | Print the active configuration. |
| Exit | Quit. |

## 7. Updating Configuration

Three ways:

1. Pick "Reconfigure" or "Quick Config" inside the program.
2. Edit `./config/instance_config.toml` by hand.
3. Delete the file and run again to regenerate from scratch.

Invalid items in the config file (placeholders, malformed OCIDs, fetch
failures) are silently ignored — the corresponding prompt falls back to its
default. You do not need to delete the config to recover from a typo.
