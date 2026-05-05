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

For OCI specifically, auth is provided through the `[oci]` section of
`./config/config` (see the Oracle guide for the field list).

## 4. Run

```bash
cargo run
```

If `./config/config` does not exist (or fails to parse), the configuration
wizard runs automatically. (If you upgraded from an earlier version that
used split files, they are migrated on first run — see
[Legacy Migration](configuration.md#legacy-migration).)

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

## 7. Non-Interactive CLI

For scripting / cron / sniping in the background, every menu option also has
a direct subcommand. When a subcommand is given the interactive menu is
skipped and the program exits when the action finishes. The config file
(`./config/config`) must already exist — these commands will not launch the
wizard automatically.

```bash
cloud-manage --help                    # list commands
cloud-manage show-config               # print the active configuration
cloud-manage create                    # launch one instance
cloud-manage refresh-ip                # choose an instance and refresh public IPv4
cloud-manage refresh-ip ocid1.instance.oc1..xxx  # optional: script with an OCID
cloud-manage snipe                     # retry until success (uses [snipe] from config)
cloud-manage snipe --min-delay 3 --max-delay 10 --max-attempts 100
cloud-manage snipe --min-delay 3 --save  # also persist the override into config
cloud-manage reconfigure               # run the full wizard
cloud-manage quick-config              # run the instance-only wizard
cloud-manage serve                     # start the embedded web UI (see docs/web-ui.md)
cloud-manage serve --host 0.0.0.0 --port 9000
```

When invoked through `cargo run`, separate cargo's own arguments from the
binary's arguments with `--`:

```bash
cargo run -- snipe --min-delay 3 --max-delay 10
```

Running `cloud-manage` with no subcommand still drops you into the
interactive menu described above.

## 8. Updating Configuration

Three ways:

1. Pick "Reconfigure" or "Quick Config" inside the program.
2. Edit `./config/config` by hand.
3. Delete the file and run again to regenerate from scratch.

Invalid items in the config file (placeholders, malformed OCIDs, fetch
failures) are silently ignored — the corresponding prompt falls back to its
default. You do not need to delete the config to recover from a typo.
