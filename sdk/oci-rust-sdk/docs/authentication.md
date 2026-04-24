# Authentication

## FileConfigProvider

Load OCI configuration from a file.

### Methods

#### `from_file(path: &Path, profile: &str) -> Result<Self>`

Load configuration from specified file and profile.

**Parameters:**
- `path` - Path to the OCI config file
- `profile` - Profile name to use (e.g., "DEFAULT")

**Returns:** `Result<FileConfigProvider, AuthError>`

**Example:**
```rust
use oci_rust_sdk::auth::FileConfigProvider;
use std::path::Path;

let config = FileConfigProvider::from_file(
    Path::new("~/.oci/config"),
    "DEFAULT"
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

#### `private_key(&self) -> Result<RsaPrivateKey>`

Get private key for signing requests.

#### `key_id(&self) -> Result<String>`

Get full key ID (format: `{tenancy}/{user}/{fingerprint}`).

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
