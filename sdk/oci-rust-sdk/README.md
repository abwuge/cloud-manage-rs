# OCI Rust SDK - User Principal Authentication

## Features Implemented

✅ **Configuration Provider Trait** - Define authentication interface  
✅ **File Configuration Provider** - Read from `~/.oci/config`  
✅ **Request Signer** - Sign HTTP requests with RSA-SHA256  
✅ **Error Handling** - Comprehensive error types

## Usage Example

```rust
use oci_rust_sdk::auth::{FileConfigProvider, RequestSigner};

// Load configuration from default location (~/.oci/config)
let config = FileConfigProvider::new()?;

// Or load from specific profile
let config = FileConfigProvider::from_profile("PRODUCTION")?;

// Create request signer
let signer = RequestSigner::new(&config)?;

// Sign a request
let auth_header = signer.sign_request(
    "GET",
    "/20160918/instances",
    "iaas.us-ashburn-1.oraclecloud.com",
    None,  // No body for GET
    &[],   // No additional headers
)?;

println!("Authorization: {}", auth_header);
```

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

## Module Structure

```
src/auth/
├── mod.rs           - Module exports
├── config.rs        - ConfigurationProvider trait
├── error.rs         - Error types
├── file_config.rs   - File-based config provider
└── signer.rs        - HTTP request signer
```

## Next Steps

- Add HTTP client integration
- Implement additional auth methods (Instance Principal, etc.)
- Add comprehensive tests
- Create service-specific clients (Compute, DNS, etc.)
