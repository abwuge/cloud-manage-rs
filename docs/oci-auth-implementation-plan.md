# OCI 认证实现计划

## 1. 认证流程概述

### 1.1 认证方式
OCI Go SDK 支持多种认证方式：
- **User Principal (API Key)**: 使用配置文件 + RSA 私钥
- **Instance Principal**: 实例元数据服务认证
- **Resource Principal**: 资源主体认证（Functions/Container Instances）
- **Session Token**: 基于会话令牌的认证
- **Delegation Token**: 委托令牌认证

**推荐首先实现**: User Principal (API Key) 认证，这是最常用的方式。

### 1.2 配置文件格式
默认位置: `~/.oci/config`

```ini
[DEFAULT]
user=ocid1.user.oc1..aaaaaaaa...
fingerprint=aa:bb:cc:dd:ee:ff:00:11:22:33:44:55:66:77:88:99
key_file=~/.oci/oci_api_key.pem
tenancy=ocid1.tenancy.oc1..aaaaaaaa...
region=us-ashburn-1
```

必需字段：
- `user`: 用户 OCID
- `fingerprint`: 公钥指纹（SHA256，冒号分隔的十六进制）
- `key_file`: 私钥文件路径
- `tenancy`: 租户 OCID
- `region`: 区域标识符

可选字段：
- `pass_phrase`: 私钥密码
- `security_token_file`: 会话令牌文件路径
- `delegation_token_file`: 委托令牌文件路径

### 1.3 请求签名流程

OCI 使用 HTTP 签名方案（基于 RFC 草案）：

1. **构建签名字符串**
   ```
   (request-target): get /20160918/instances
   host: iaas.us-ashburn-1.oraclecloud.com
   date: Thu, 05 Jan 2014 21:31:40 GMT
   x-content-sha256: <body的SHA256>
   content-type: application/json
   content-length: 316
   ```

2. **计算签名**
   - 使用 RSA-SHA256 对签名字符串进行签名
   - Base64 编码签名结果

3. **添加 Authorization 头**
   ```
   Authorization: Signature version="1",
                  headers="date (request-target) host",
                  keyId="ocid1.tenancy.../ocid1.user.../aa:bb:cc...",
                  algorithm="rsa-sha256",
                  signature="<base64_signature>"
   ```

**签名头分类**：
- **通用头** (Generic Headers): `date`, `(request-target)`, `host`
- **Body 头** (Body Headers): `content-length`, `content-type`, `x-content-sha256`

**签名规则**：
- GET/HEAD/DELETE: 只签名通用头
- POST/PUT/PATCH: 签名通用头 + Body 头
- Body SHA256: 计算请求体的 SHA256 哈希，设置 `x-content-sha256` 头

### 1.4 KeyID 格式
```
{tenancy_ocid}/{user_ocid}/{key_fingerprint}
```
示例: `ocid1.tenancy.oc1..aaa/ocid1.user.oc1..bbb/aa:bb:cc:dd:ee:ff:00:11:22:33:44:55:66:77:88:99`

---

## 2. Go SDK 认证模块结构

### 2.1 核心接口

```go
// common/configuration.go
type ConfigurationProvider interface {
    KeyProvider
    TenancyOCID() (string, error)
    UserOCID() (string, error)
    KeyFingerprint() (string, error)
    Region() (string, error)
    AuthType() (AuthConfig, error)
}

type KeyProvider interface {
    PrivateRSAKey() (*rsa.PrivateKey, error)
    KeyID() (string, error)
}

// common/http_signer.go
type HTTPRequestSigner interface {
    Sign(r *http.Request) error
}
```

### 2.2 实现类型

1. **fileConfigurationProvider**: 从配置文件读取
2. **rawConfigurationProvider**: 程序化配置
3. **environmentConfigurationProvider**: 从环境变量读取
4. **instancePrincipalConfigurationProvider**: 实例主体认证
5. **resourcePrincipalConfigurationProvider**: 资源主体认证

### 2.3 签名器实现

```go
// common/http_signer.go
type ociRequestSigner struct {
    KeyProvider    KeyProvider
    GenericHeaders []string  // ["date", "(request-target)", "host"]
    BodyHeaders    []string  // ["content-length", "content-type", "x-content-sha256"]
    ShouldHashBody SignerBodyHashPredicate
}

func (signer ociRequestSigner) Sign(request *http.Request) error {
    // 1. 计算 body hash (如果需要)
    // 2. 构建签名字符串
    // 3. 使用 RSA-SHA256 签名
    // 4. 添加 Authorization 头
}
```

---

## 3. Rust 实现方案

### 3.1 所需 Crates

```toml
[dependencies]
# HTTP 客户端
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

# 序列化/反序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 加密
rsa = "0.9"
sha2 = "0.10"
base64 = "0.22"

# 配置解析
ini = "1.3"  # 或 toml = "0.8"

# 错误处理
thiserror = "1.0"
anyhow = "1.0"

# 异步运行时
tokio = { version = "1", features = ["full"] }

# 日期时间
chrono = "0.4"

# 路径展开
shellexpand = "3.1"
```

### 3.2 模块结构

```
src/
├── auth/
│   ├── mod.rs                    # 认证模块入口
│   ├── config.rs                 # 配置提供者 trait 和实现
│   ├── signer.rs                 # HTTP 请求签名器
│   ├── file_config.rs            # 文件配置提供者
│   ├── raw_config.rs             # 程序化配置提供者
│   ├── env_config.rs             # 环境变量配置提供者
│   └── error.rs                  # 认证错误类型
├── client/
│   ├── mod.rs                    # HTTP 客户端
│   └── base_client.rs            # 基础客户端实现
└── providers/
    └── ...                       # 各服务提供者
```

### 3.3 核心 Trait 定义

```rust
// src/auth/config.rs
use rsa::RsaPrivateKey;
use std::error::Error;

/// 密钥提供者 trait
pub trait KeyProvider {
    /// 获取 RSA 私钥
    fn private_rsa_key(&self) -> Result<RsaPrivateKey, Box<dyn Error>>;
    
    /// 获取 Key ID (格式: tenancy/user/fingerprint)
    fn key_id(&self) -> Result<String, Box<dyn Error>>;
}

/// 配置提供者 trait
pub trait ConfigurationProvider: KeyProvider {
    /// 获取租户 OCID
    fn tenancy_ocid(&self) -> Result<String, Box<dyn Error>>;
    
    /// 获取用户 OCID
    fn user_ocid(&self) -> Result<String, Box<dyn Error>>;
    
    /// 获取密钥指纹
    fn key_fingerprint(&self) -> Result<String, Box<dyn Error>>;
    
    /// 获取区域
    fn region(&self) -> Result<String, Box<dyn Error>>;
}

/// 认证类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthType {
    UserPrincipal,
    InstancePrincipal,
    ResourcePrincipal,
    SessionToken,
}
```

### 3.4 文件配置提供者实现

```rust
// src/auth/file_config.rs
use super::config::{ConfigurationProvider, KeyProvider};
use ini::Ini;
use rsa::RsaPrivateKey;
use rsa::pkcs8::DecodePrivateKey;
use std::path::{Path, PathBuf};
use std::fs;

pub struct FileConfigurationProvider {
    config_path: PathBuf,
    profile: String,
    private_key_password: Option<String>,
    // 缓存解析后的配置
    cached_config: Option<ConfigData>,
}

struct ConfigData {
    user_ocid: String,
    tenancy_ocid: String,
    fingerprint: String,
    key_file_path: String,
    region: String,
    pass_phrase: Option<String>,
}

impl FileConfigurationProvider {
    pub fn new(config_path: impl AsRef<Path>, profile: &str) -> Result<Self, Box<dyn Error>> {
        let config_path = shellexpand::tilde(config_path.as_ref().to_str().unwrap())
            .into_owned()
            .into();
        
        Ok(Self {
            config_path,
            profile: profile.to_string(),
            private_key_password: None,
            cached_config: None,
        })
    }
    
    pub fn with_password(mut self, password: String) -> Self {
        self.private_key_password = Some(password);
        self
    }
    
    fn load_config(&mut self) -> Result<&ConfigData, Box<dyn Error>> {
        if self.cached_config.is_none() {
            let ini = Ini::load_from_file(&self.config_path)?;
            let section = ini.section(Some(&self.profile))
                .ok_or("Profile not found")?;
            
            self.cached_config = Some(ConfigData {
                user_ocid: section.get("user")
                    .ok_or("Missing 'user' field")?.to_string(),
                tenancy_ocid: section.get("tenancy")
                    .ok_or("Missing 'tenancy' field")?.to_string(),
                fingerprint: section.get("fingerprint")
                    .ok_or("Missing 'fingerprint' field")?.to_string(),
                key_file_path: section.get("key_file")
                    .ok_or("Missing 'key_file' field")?.to_string(),
                region: section.get("region")
                    .ok_or("Missing 'region' field")?.to_string(),
                pass_phrase: section.get("pass_phrase").map(|s| s.to_string()),
            });
        }
        
        Ok(self.cached_config.as_ref().unwrap())
    }
}

impl KeyProvider for FileConfigurationProvider {
    fn private_rsa_key(&self) -> Result<RsaPrivateKey, Box<dyn Error>> {
        let config = self.load_config()?;
        let key_path = shellexpand::tilde(&config.key_file_path).into_owned();
        let pem_content = fs::read_to_string(key_path)?;
        
        // 根据是否有密码选择解析方式
        let key = if let Some(password) = &self.private_key_password
            .or(config.pass_phrase.clone()) 
        {
            RsaPrivateKey::from_pkcs8_encrypted_pem(&pem_content, password.as_bytes())?
        } else {
            RsaPrivateKey::from_pkcs8_pem(&pem_content)?
        };
        
        Ok(key)
    }
    
    fn key_id(&self) -> Result<String, Box<dyn Error>> {
        let config = self.load_config()?;
        Ok(format!(
            "{}/{}/{}",
            config.tenancy_ocid,
            config.user_ocid,
            config.fingerprint
        ))
    }
}

impl ConfigurationProvider for FileConfigurationProvider {
    fn tenancy_ocid(&self) -> Result<String, Box<dyn Error>> {
        Ok(self.load_config()?.tenancy_ocid.clone())
    }
    
    fn user_ocid(&self) -> Result<String, Box<dyn Error>> {
        Ok(self.load_config()?.user_ocid.clone())
    }
    
    fn key_fingerprint(&self) -> Result<String, Box<dyn Error>> {
        Ok(self.load_config()?.fingerprint.clone())
    }
    
    fn region(&self) -> Result<String, Box<dyn Error>> {
        Ok(self.load_config()?.region.clone())
    }
}
```

### 3.5 HTTP 请求签名器实现

```rust
// src/auth/signer.rs
use super::config::KeyProvider;
use reqwest::Request;
use rsa::signature::{SignatureEncoding, Signer};
use rsa::pkcs1v15::SigningKey;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;

pub struct OciRequestSigner<P: KeyProvider> {
    key_provider: P,
    generic_headers: Vec<String>,
    body_headers: Vec<String>,
}

impl<P: KeyProvider> OciRequestSigner<P> {
    pub fn new(key_provider: P) -> Self {
        Self {
            key_provider,
            generic_headers: vec![
                "date".to_string(),
                "(request-target)".to_string(),
                "host".to_string(),
            ],
            body_headers: vec![
                "content-length".to_string(),
                "content-type".to_string(),
                "x-content-sha256".to_string(),
            ],
        }
    }
    
    pub fn sign(&self, request: &mut Request) -> Result<(), Box<dyn Error>> {
        // 1. 设置 Date 头
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        request.headers_mut().insert("date", date.parse()?);
        
        // 2. 计算 body hash (如果需要)
        if self.should_hash_body(request) {
            self.calculate_body_hash(request)?;
        }
        
        // 3. 构建签名字符串
        let signing_string = self.build_signing_string(request)?;
        
        // 4. 计算签名
        let signature = self.compute_signature(&signing_string)?;
        
        // 5. 构建 Authorization 头
        let headers_list = self.get_signing_headers(request).join(" ");
        let key_id = self.key_provider.key_id()?;
        let auth_header = format!(
            r#"Signature version="1",headers="{}",keyId="{}",algorithm="rsa-sha256",signature="{}""#,
            headers_list, key_id, signature
        );
        
        request.headers_mut().insert("authorization", auth_header.parse()?);
        
        Ok(())
    }
    
    fn should_hash_body(&self, request: &Request) -> bool {
        matches!(
            request.method().as_str(),
            "POST" | "PUT" | "PATCH"
        )
    }
    
    fn calculate_body_hash(&self, request: &mut Request) -> Result<(), Box<dyn Error>> {
        if let Some(body) = request.body() {
            let body_bytes = body.as_bytes().unwrap_or(&[]);
            let mut hasher = Sha256::new();
            hasher.update(body_bytes);
            let hash = hasher.finalize();
            let hash_b64 = general_purpose::STANDARD.encode(hash);
            
            request.headers_mut().insert("x-content-sha256", hash_b64.parse()?);
            request.headers_mut().insert(
                "content-length",
                body_bytes.len().to_string().parse()?
            );
        }
        Ok(())
    }
    
    fn build_signing_string(&self, request: &Request) -> Result<String, Box<dyn Error>> {
        let mut parts = Vec::new();
        
        for header in self.get_signing_headers(request) {
            let value = if header == "(request-target)" {
                format!(
                    "{} {}",
                    request.method().as_str().to_lowercase(),
                    request.url().path()
                )
            } else {
                request.headers()
                    .get(&header)
                    .ok_or(format!("Missing header: {}", header))?
                    .to_str()?
                    .to_string()
            };
            
            parts.push(format!("{}: {}", header, value));
        }
        
        Ok(parts.join("\n"))
    }
    
    fn get_signing_headers(&self, request: &Request) -> Vec<String> {
        let mut headers = self.generic_headers.clone();
        
        if self.should_hash_body(request) {
            headers.extend(self.body_headers.clone());
        }
        
        headers
    }
    
    fn compute_signature(&self, signing_string: &str) -> Result<String, Box<dyn Error>> {
        let private_key = self.key_provider.private_rsa_key()?;
        let signing_key = SigningKey::<Sha256>::new(private_key);
        
        let mut hasher = Sha256::new();
        hasher.update(signing_string.as_bytes());
        let hashed = hasher.finalize();
        
        let signature = signing_key.sign(&hashed);
        Ok(general_purpose::STANDARD.encode(signature.to_bytes()))
    }
}
```

### 3.6 基础 HTTP 客户端

```rust
// src/client/base_client.rs
use crate::auth::{ConfigurationProvider, OciRequestSigner};
use reqwest::{Client, Request, Response};

pub struct BaseClient<P: ConfigurationProvider> {
    http_client: Client,
    signer: OciRequestSigner<P>,
    region: String,
    base_path: String,
}

impl<P: ConfigurationProvider> BaseClient<P> {
    pub fn new(config_provider: P) -> Result<Self, Box<dyn Error>> {
        let region = config_provider.region()?;
        let signer = OciRequestSigner::new(config_provider);
        
        Ok(Self {
            http_client: Client::new(),
            signer,
            region,
            base_path: String::new(),
        })
    }
    
    pub async fn call(&self, mut request: Request) -> Result<Response, Box<dyn Error>> {
        // 签名请求
        self.signer.sign(&mut request)?;
        
        // 发送请求
        let response = self.http_client.execute(request).await?;
        
        Ok(response)
    }
    
    pub fn set_region(&mut self, region: String) {
        self.region = region;
    }
}
```

---

## 4. 实现步骤

### 阶段 1: 核心认证基础设施 (1-2 天)
1. ✅ 定义 `KeyProvider` 和 `ConfigurationProvider` traits
2. ✅ 实现 `FileConfigurationProvider`
   - INI 配置文件解析
   - 私钥加载（支持加密/非加密）
   - 路径展开（`~` 支持）
3. ✅ 实现 `RawConfigurationProvider`（程序化配置）
4. ✅ 编写单元测试

### 阶段 2: 请求签名实现 (2-3 天)
1. ✅ 实现 `OciRequestSigner`
   - 签名字符串构建
   - RSA-SHA256 签名
   - Authorization 头生成
2. ✅ 实现 body hash 计算
3. ✅ 处理不同 HTTP 方法的签名逻辑
4. ✅ 编写签名测试（对比 Go SDK 的测试用例）

### 阶段 3: HTTP 客户端集成 (1-2 天)
1. ✅ 实现 `BaseClient`
2. ✅ 集成 `reqwest` HTTP 客户端
3. ✅ 实现请求拦截器（自动签名）
4. ✅ 错误处理和重试逻辑

### 阶段 4: 扩展认证方式 (可选，3-5 天)
1. ⏸️ 实现 `EnvironmentConfigurationProvider`
2. ⏸️ 实现 Instance Principal 认证
3. ⏸️ 实现 Resource Principal 认证
4. ⏸️ 实现 Session Token 认证

### 阶段 5: 测试和文档 (2-3 天)
1. ✅ 集成测试（真实 API 调用）
2. ✅ 性能测试
3. ✅ 编写使用文档和示例
4. ✅ API 文档生成

---

## 5. 关键实现细节

### 5.1 私钥解析
- 支持 PEM 格式（PKCS#1 和 PKCS#8）
- 支持加密私钥（使用密码解密）
- 使用 `rsa` crate 的 `DecodePrivateKey` trait

### 5.2 签名字符串格式
```
date: Thu, 05 Jan 2014 21:31:40 GMT
(request-target): get /20160918/instances
host: iaas.us-ashburn-1.oraclecloud.com
x-content-sha256: <base64_hash>
content-type: application/json
content-length: 316
```

注意事项：
- 头名称小写
- `(request-target)` 格式: `{method} {path}`
- 每行格式: `{header}: {value}`
- 使用 `\n` 连接

### 5.3 区域端点构建
```rust
fn endpoint_for_service(&self, service: &str) -> String {
    format!("https://{}.{}.oraclecloud.com", service, self.region)
}
```

### 5.4 错误处理
```rust
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Configuration file not found: {0}")]
    ConfigNotFound(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Invalid private key: {0}")]
    InvalidPrivateKey(String),
    
    #[error("Signing failed: {0}")]
    SigningFailed(String),
}
```

---

## 6. 测试策略

### 6.1 单元测试
- 配置文件解析
- 私钥加载
- 签名字符串构建
- 签名计算

### 6.2 集成测试
- 使用真实配置调用 OCI API
- 验证签名正确性
- 测试错误处理

### 6.3 性能测试
- 签名性能
- 配置缓存效果
- 并发请求处理

---

## 7. 参考资源

### 官方文档
- [OCI SDK Configuration](https://docs.oracle.com/iaas/Content/API/Concepts/sdkconfig.htm)
- [Request Signatures](https://docs.oracle.com/iaas/Content/API/Concepts/signingrequests.htm)
- [OCI Go SDK](https://github.com/oracle/oci-go-sdk)

### Go SDK 关键文件
- `common/configuration.go`: 配置提供者实现
- `common/http_signer.go`: 请求签名实现
- `common/auth/`: 各种认证方式实现
- `common/client.go`: 基础客户端

### Rust Crates 文档
- [reqwest](https://docs.rs/reqwest/)
- [rsa](https://docs.rs/rsa/)
- [sha2](https://docs.rs/sha2/)
- [ini](https://docs.rs/ini/)

---

## 8. 风险和注意事项

### 8.1 兼容性
- 确保签名算法与 OCI 完全兼容
- 测试不同区域的端点格式
- 验证 OCID 格式

### 8.2 安全性
- 私钥内存安全（使用 `zeroize` crate）
- 避免日志中泄露敏感信息
- 配置文件权限检查

### 8.3 性能
- 配置缓存策略
- 私钥缓存（避免重复解析）
- HTTP 连接池复用

---

## 9. 下一步行动

1. **立即开始**: 实现阶段 1（核心认证基础设施）
2. **并行进行**: 准备测试环境和测试数据
3. **持续验证**: 每个阶段完成后与 Go SDK 对比测试
4. **文档同步**: 边实现边编写文档和示例

预计总工时: **10-15 天**（包含测试和文档）
