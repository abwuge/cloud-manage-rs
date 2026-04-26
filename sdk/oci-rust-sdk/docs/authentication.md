# Authentication

## FileConfigProvider

Load OCI configuration from a file.

### Methods

#### `new() -> Result<Self>`

Load from the default location (`~/.oci/config`) and the `DEFAULT` profile.

#### `from_profile(profile: &str) -> Result<Self>`

Load from the default location (`~/.oci/config`) using a specific profile.

#### `from_file(path: &Path, profile: &str) -> Result<Self>`

Load configuration from a specific file and profile.

**Parameters:**
- `path` - Path to the OCI config file
- `profile` - Profile name to use (e.g., `"DEFAULT"`)

**Returns:** `Result<FileConfigProvider, AuthError>`

**Example:**
```rust
use oci_rust_sdk::auth::FileConfigProvider;
use std::path::Path;

let config = FileConfigProvider::from_file(
    Path::new("~/.oci/config"),
    "DEFAULT",
)?;
```

## ConfigurationProvider

Trait for providing OCI configuration.

### Methods

#### `user_id(&self) -> Result<String>`

Get user OCID.

#### `tenancy_id(&self) -> Result<String>`

Get tenancy OCID.

#### `region(&self) -> Result<String>`

Get region identifier.

#### `fingerprint(&self) -> Result<String>`

Get key fingerprint.

#### `private_key(&self) -> Result<String>`

Get the PEM-encoded private key used to sign requests. The signer (`RequestSigner`) parses this PEM internally; callers do not need to convert it themselves.

#### `passphrase(&self) -> Result<Option<String>>`

Get the passphrase protecting the private key, or `None` if the key is unencrypted. The default trait impl returns `None`; `FileConfigProvider` overrides it to read the optional `passphrase` field from the config file.

#### `key_id(&self) -> Result<String>`

Get the full key ID (format: `{tenancy}/{user}/{fingerprint}`). The trait provides a default implementation built from the three accessors above.

## Configuration File Format

`~/.oci/config`:
```ini
[DEFAULT]
user=ocid1.user.oc1..aaaaaaaa...
fingerprint=aa:bb:cc:dd:ee:ff:00:11:22:33:44:55:66:77:88:99
key_file=~/.oci/oci_api_key.pem
tenancy=ocid1.tenancy.oc1..aaaaaaaa...
region=us-ashburn-1
```
