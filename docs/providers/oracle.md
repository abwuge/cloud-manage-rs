# Oracle Cloud Infrastructure (OCI)

This guide covers OCI-specific setup, supported resources and tips. For the
generic flow (wizard, menu, file layout) see
[Getting Started](../getting-started.md) and
[Configuration Reference](../configuration.md).

## Auth Setup

The program reads OCI auth from `./config/oci_config` in standard OCI INI
format:

```ini
[DEFAULT]
user        = ocid1.user.oc1..your-user-id
fingerprint = your-fingerprint
tenancy     = ocid1.tenancy.oc1..your-tenancy-id
region      = your-region
key_file    = ./config/oci_api_key.pem
```

Generate / fetch these values from the OCI Console
(Identity & Security → Users → API Keys). Place the matching private key at
`./config/oci_api_key.pem`.

## Supported Resources

| Resource | Action |
| --- | --- |
| Compute instance | Create, wait until `RUNNING` |
| Image | List by distribution + major version |
| Subnet | List + select |
| Availability Domain | List + select |

The wizard fetches lists via the bundled
[OCI Rust SDK](../../sdk/oci-rust-sdk).

## Always Free Targets

The program is currently optimised for Always Free shapes:

### AMD Micro (`VM.Standard.E2.1.Micro`)

- Up to 2 instances per tenancy.
- 1/8 OCPU, 1 GB memory (fixed).

### ARM Flex (`VM.Standard.A1.Flex`)

- 3,000 OCPU-hours and 18,000 GB-hours per month free.
- Equivalent to 4 OCPU and 24 GB memory total across all ARM instances.
- Configurable 1–4 OCPU per instance (1 OCPU = 6 GB memory).

### Block Storage

- 200 GB free.
- Minimum boot volume 47 GB.

## Configuration Block

The `[oracle]` section in `instance_config.toml`:

| Field | Description |
| --- | --- |
| `compartment_id` | Compartment OCID. Defaults to the tenancy from `oci_config`. |
| `availability_domain` | AD name, e.g. `Uocm:PHX-AD-1`. |
| `subnet_id` | Subnet OCID; the subnet's CIDR (and IPv6 CIDR if any) is read at create time. |
| `image_id_amd` | Image OCID used when `instance_type = "amd"`. |
| `image_id_arm` | Image OCID used when `instance_type = "arm"`. |
| `ssh_public_key` | Authorized key injected into the instance. |

## Resource Discovery

These items are picked from API listings inside the wizard, so you usually
do not need to copy OCIDs from the console:

- **Compartment** — the wizard's default is the tenancy OCID read from
  `oci_config`. Override only if you want a sub-compartment.
- **Availability Domain** — listed for the chosen compartment.
- **Subnet** — listed for the chosen compartment, with CIDR shown inline.
  If you have no VCN/subnet yet, create one first via the OCI Console:
  Networking → Virtual Cloud Networks → "Start VCN Wizard" →
  "Create VCN with Internet Connectivity".
- **Images** — pick distribution → major version. The latest matching AMD
  and ARM images are auto-selected.

Currently supported distributions:

- Oracle Linux
- Canonical Ubuntu
- Windows Server
- CentOS
- Red Hat Enterprise Linux _(subscription required)_

## SSH Public Key

If you do not already have a key:

```bash
ssh-keygen -t ed25519 -f ~/.ssh/oci_key
cat ~/.ssh/oci_key.pub
```

Paste the entire public-key line into the wizard.

## Address Entry UX

For private IPv4 / custom IPv6 the wizard locks the subnet prefix in front
and only takes the host suffix:

- `Private IPv4 address (Subnet: 10.0.0.0/16) › 10.0.` then type `0.11`
- `IPv6 address (Subnet: 2001:db8:1234:5678::/64) › 2001:db8:1234:5678:`
  then type `11`

## Connecting to a New Instance

After the program reports `RUNNING`, find the public IP in the OCI Console
(Compute → Instances → _your instance_) and connect:

```bash
ssh -i ~/.ssh/oci_key <user>@<public-ip>
```

Default users:

| Image | User |
| --- | --- |
| Oracle Linux | `opc` |
| Canonical Ubuntu | `ubuntu` |

## Troubleshooting

### `Out of host capacity`

Always Free capacity in the chosen AD is exhausted. Options:

1. Try a different availability domain.
2. Retry later.
3. Upgrade to a paid account.

### Terminating instances

Currently from the OCI Console: Compute → Instances → overflow menu (`⋮`)
→ Terminate. A native command is on the roadmap.

### Tags

The program automatically applies the `managed-by: cloud-manage-rs` freeform
tag to every instance it creates.
