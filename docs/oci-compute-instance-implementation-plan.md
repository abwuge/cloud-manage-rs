# OCI Compute Instance 创建 API 实现方案

## API 端点

**POST** `/20160918/instances/`

基础 URL: `https://iaas.{region}.oraclecloud.com`

## 核心数据结构

### 1. LaunchInstanceDetails (请求体)

```rust
pub struct LaunchInstanceDetails {
    // 必需参数
    pub availability_domain: String,        // 可用域，如 "Uocm:PHX-AD-1"
    pub compartment_id: String,             // 隔间 OCID
    
    // 核心配置
    pub shape: String,                      // 实例形状，如 "VM.Standard2.1"
    pub source_details: InstanceSourceDetails, // 镜像或启动卷
    
    // 网络配置
    pub create_vnic_details: Option<CreateVnicDetails>,
    pub subnet_id: Option<String>,          // 子网 OCID
    
    // 可选参数
    pub display_name: Option<String>,       // 显示名称
    pub hostname_label: Option<String>,     // 主机名标签
    pub metadata: Option<HashMap<String, String>>, // 自定义元数据
    pub extended_metadata: Option<HashMap<String, serde_json::Value>>,
    
    // 高级配置
    pub shape_config: Option<LaunchInstanceShapeConfigDetails>,
    pub agent_config: Option<LaunchInstanceAgentConfigDetails>,
    pub launch_options: Option<LaunchOptions>,
    pub instance_options: Option<InstanceOptions>,
    pub availability_config: Option<LaunchInstanceAvailabilityConfigDetails>,
    
    // 标签
    pub freeform_tags: Option<HashMap<String, String>>,
    pub defined_tags: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
    
    // 其他
    pub fault_domain: Option<String>,
    pub dedicated_vm_host_id: Option<String>,
    pub capacity_reservation_id: Option<String>,
    pub ipxe_script: Option<String>,
    pub is_pv_encryption_in_transit_enabled: Option<bool>,
}
```

### 2. InstanceSourceDetails (枚举)

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "sourceType")]
pub enum InstanceSourceDetails {
    #[serde(rename = "image")]
    Image {
        image_id: String,
        boot_volume_size_in_gbs: Option<i64>,
        kms_key_id: Option<String>,
        boot_volume_vpus_per_gb: Option<i64>,
    },
    #[serde(rename = "bootVolume")]
    BootVolume {
        boot_volume_id: String,
    },
}
```

### 3. CreateVnicDetails

```rust
pub struct CreateVnicDetails {
    pub subnet_id: String,
    pub assign_public_ip: Option<bool>,
    pub assign_private_dns_record: Option<bool>,
    pub display_name: Option<String>,
    pub hostname_label: Option<String>,
    pub private_ip: Option<String>,
    pub skip_source_dest_check: Option<bool>,
    pub nsg_ids: Option<Vec<String>>,
    pub freeform_tags: Option<HashMap<String, String>>,
    pub defined_tags: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
}
```

### 4. LaunchInstanceShapeConfigDetails

```rust
pub struct LaunchInstanceShapeConfigDetails {
    pub ocpus: Option<f32>,                 // OCPU 数量
    pub memory_in_gbs: Option<f32>,         // 内存大小 (GB)
    pub baseline_ocpu_utilization: Option<String>, // 基线 OCPU 利用率
    pub nvmes: Option<i32>,                 // NVMe 驱动器数量
}
```

### 5. Instance (响应体)

```rust
pub struct Instance {
    pub id: String,                         // 实例 OCID
    pub compartment_id: String,
    pub availability_domain: String,
    pub lifecycle_state: InstanceLifecycleState,
    pub shape: String,
    pub region: String,
    
    pub display_name: Option<String>,
    pub time_created: String,               // RFC3339 格式
    pub image_id: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
    pub extended_metadata: Option<HashMap<String, serde_json::Value>>,
    
    pub shape_config: Option<InstanceShapeConfig>,
    pub source_details: Option<InstanceSourceDetails>,
    pub agent_config: Option<InstanceAgentConfig>,
    
    pub freeform_tags: Option<HashMap<String, String>>,
    pub defined_tags: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
    
    pub fault_domain: Option<String>,
    pub dedicated_vm_host_id: Option<String>,
    pub launch_mode: Option<String>,
    pub launch_options: Option<LaunchOptions>,
    pub instance_options: Option<InstanceOptions>,
    pub availability_config: Option<InstanceAvailabilityConfig>,
    pub preemptible_instance_config: Option<PreemptibleInstanceConfig>,
    
    pub time_maintenance_reboot_due: Option<String>,
}
```

### 6. InstanceLifecycleState (枚举)

```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InstanceLifecycleState {
    #[serde(rename = "PROVISIONING")]
    Provisioning,
    #[serde(rename = "RUNNING")]
    Running,
    #[serde(rename = "STARTING")]
    Starting,
    #[serde(rename = "STOPPING")]
    Stopping,
    #[serde(rename = "STOPPED")]
    Stopped,
    #[serde(rename = "CREATING_IMAGE")]
    CreatingImage,
    #[serde(rename = "TERMINATING")]
    Terminating,
    #[serde(rename = "TERMINATED")]
    Terminated,
}
```

## HTTP 请求/响应

### 请求

```http
POST /20160918/instances/ HTTP/1.1
Host: iaas.us-phoenix-1.oraclecloud.com
Content-Type: application/json
Date: Thu, 24 Apr 2026 12:00:00 GMT
Authorization: Signature version="1",headers="date (request-target) host content-length content-type x-content-sha256",keyId="...",algorithm="rsa-sha256",signature="..."
opc-retry-token: unique-token-123
opc-request-id: optional-request-id

{
  "availabilityDomain": "Uocm:PHX-AD-1",
  "compartmentId": "ocid1.compartment.oc1...",
  "shape": "VM.Standard2.1",
  "sourceDetails": {
    "sourceType": "image",
    "imageId": "ocid1.image.oc1.phx...",
    "bootVolumeSizeInGBs": 50
  },
  "createVnicDetails": {
    "subnetId": "ocid1.subnet.oc1.phx...",
    "assignPublicIp": true
  },
  "displayName": "my-instance",
  "metadata": {
    "ssh_authorized_keys": "ssh-rsa AAAA..."
  }
}
```

### 响应 (200 OK)

```http
HTTP/1.1 200 OK
Content-Type: application/json
etag: "W/\"datetime'2026-04-24T12%3A00%3A00.000Z'\""
opc-request-id: unique-request-id
opc-work-request-id: ocid1.workrequest.oc1.phx...

{
  "id": "ocid1.instance.oc1.phx...",
  "compartmentId": "ocid1.compartment.oc1...",
  "availabilityDomain": "Uocm:PHX-AD-1",
  "lifecycleState": "PROVISIONING",
  "shape": "VM.Standard2.1",
  "region": "phx",
  "displayName": "my-instance",
  "timeCreated": "2026-04-24T12:00:00.000Z",
  "imageId": "ocid1.image.oc1.phx...",
  "metadata": {
    "ssh_authorized_keys": "ssh-rsa AAAA..."
  }
}
```

## Go SDK 参考实现

### ComputeClient 结构

```go
type ComputeClient struct {
    common.BaseClient
    config *common.ConfigurationProvider
}

func NewComputeClientWithConfigurationProvider(
    configProvider common.ConfigurationProvider,
) (client ComputeClient, err error)
```

### LaunchInstance 方法

```go
func (client ComputeClient) LaunchInstance(
    ctx context.Context,
    request LaunchInstanceRequest,
) (response LaunchInstanceResponse, err error) {
    var ociResponse common.OCIResponse
    policy := common.NoRetryPolicy()
    if client.RetryPolicy() != nil {
        policy = *client.RetryPolicy()
    }
    if request.RetryPolicy() != nil {
        policy = *request.RetryPolicy()
    }
    
    if !(request.OpcRetryToken != nil && *request.OpcRetryToken != "") {
        request.OpcRetryToken = common.String(common.RetryToken())
    }
    
    ociResponse, err = common.Retry(ctx, request, client.launchInstance, policy)
    // ... 错误处理
    return
}

func (client ComputeClient) launchInstance(
    ctx context.Context,
    request common.OCIRequest,
    binaryReqBody *common.OCIReadSeekCloser,
    extraHeaders map[string]string,
) (common.OCIResponse, error) {
    httpRequest, err := request.HTTPRequest(
        http.MethodPost,
        "/instances/",
        binaryReqBody,
        extraHeaders,
    )
    
    var response LaunchInstanceResponse
    var httpResponse *http.Response
    httpResponse, err = client.Call(ctx, &httpRequest)
    defer common.CloseBodyIfValid(httpResponse)
    
    err = common.UnmarshalResponse(httpResponse, &response)
    return response, err
}
```

## Rust SDK 实现步骤

### 1. 定义数据结构 (`src/compute/models.rs`)

```rust
// 请求/响应模型
pub mod models {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    
    // LaunchInstanceDetails, Instance, 等结构体定义
    // (参考上面的核心数据结构)
}
```

### 2. 实现 ComputeClient (`src/compute/client.rs`)

```rust
use crate::auth::Signer;
use crate::error::OciError;
use reqwest::Client as HttpClient;

pub struct ComputeClient {
    http_client: HttpClient,
    signer: Box<dyn Signer>,
    endpoint: String,  // https://iaas.{region}.oraclecloud.com
}

impl ComputeClient {
    pub fn new(signer: Box<dyn Signer>, region: &str) -> Self {
        Self {
            http_client: HttpClient::new(),
            signer,
            endpoint: format!("https://iaas.{}.oraclecloud.com", region),
        }
    }
    
    pub async fn launch_instance(
        &self,
        request: LaunchInstanceDetails,
    ) -> Result<Instance, OciError> {
        let url = format!("{}/20160918/instances/", self.endpoint);
        let body = serde_json::to_vec(&request)?;
        
        // 生成签名
        let headers = self.signer.sign(
            "POST",
            &url,
            &body,
            &[],
        )?;
        
        // 发送请求
        let response = self.http_client
            .post(&url)
            .headers(headers)
            .body(body)
            .send()
            .await?;
        
        // 处理响应
        if response.status().is_success() {
            let instance: Instance = response.json().await?;
            Ok(instance)
        } else {
            let error_body = response.text().await?;
            Err(OciError::ApiError(error_body))
        }
    }
    
    pub async fn get_instance(
        &self,
        instance_id: &str,
    ) -> Result<Instance, OciError> {
        let url = format!("{}/20160918/instances/{}", self.endpoint, instance_id);
        
        let headers = self.signer.sign("GET", &url, &[], &[])?;
        
        let response = self.http_client
            .get(&url)
            .headers(headers)
            .send()
            .await?;
        
        if response.status().is_success() {
            let instance: Instance = response.json().await?;
            Ok(instance)
        } else {
            let error_body = response.text().await?;
            Err(OciError::ApiError(error_body))
        }
    }
    
    pub async fn terminate_instance(
        &self,
        instance_id: &str,
    ) -> Result<(), OciError> {
        let url = format!("{}/20160918/instances/{}", self.endpoint, instance_id);
        
        let headers = self.signer.sign("DELETE", &url, &[], &[])?;
        
        let response = self.http_client
            .delete(&url)
            .headers(headers)
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            let error_body = response.text().await?;
            Err(OciError::ApiError(error_body))
        }
    }
}
```

### 3. 模块组织 (`src/compute/mod.rs`)

```rust
pub mod client;
pub mod models;

pub use client::ComputeClient;
pub use models::*;
```

### 4. 使用示例

```rust
use oci_rust_sdk::auth::UserPrincipalAuth;
use oci_rust_sdk::compute::{ComputeClient, LaunchInstanceDetails, InstanceSourceDetails};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化认证
    let auth = UserPrincipalAuth::from_config_file(None)?;
    
    // 创建 Compute 客户端
    let client = ComputeClient::new(Box::new(auth), "us-phoenix-1");
    
    // 构建启动实例请求
    let mut metadata = HashMap::new();
    metadata.insert(
        "ssh_authorized_keys".to_string(),
        "ssh-rsa AAAA...".to_string(),
    );
    
    let request = LaunchInstanceDetails {
        availability_domain: "Uocm:PHX-AD-1".to_string(),
        compartment_id: "ocid1.compartment.oc1...".to_string(),
        shape: "VM.Standard2.1".to_string(),
        source_details: InstanceSourceDetails::Image {
            image_id: "ocid1.image.oc1.phx...".to_string(),
            boot_volume_size_in_gbs: Some(50),
            kms_key_id: None,
            boot_volume_vpus_per_gb: None,
        },
        create_vnic_details: Some(CreateVnicDetails {
            subnet_id: "ocid1.subnet.oc1.phx...".to_string(),
            assign_public_ip: Some(true),
            ..Default::default()
        }),
        display_name: Some("my-instance".to_string()),
        metadata: Some(metadata),
        ..Default::default()
    };
    
    // 启动实例
    let instance = client.launch_instance(request).await?;
    println!("Instance created: {}", instance.id);
    println!("State: {:?}", instance.lifecycle_state);
    
    // 等待实例运行
    loop {
        let current = client.get_instance(&instance.id).await?;
        println!("Current state: {:?}", current.lifecycle_state);
        
        if current.lifecycle_state == InstanceLifecycleState::Running {
            println!("Instance is running!");
            break;
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
    
    Ok(())
}
```

## 关键实现要点

1. **请求签名**: 使用已实现的 `Signer` trait 对所有请求进行签名
2. **错误处理**: 解析 OCI API 错误响应,提供详细错误信息
3. **重试机制**: 实现指数退避重试策略
4. **异步支持**: 使用 `tokio` 和 `reqwest` 实现异步 HTTP 请求
5. **类型安全**: 使用 Rust 类型系统确保 API 参数正确性
6. **序列化**: 使用 `serde` 处理 JSON 序列化/反序列化
7. **生命周期管理**: 提供轮询方法等待实例状态变化

## 测试策略

1. **单元测试**: 测试数据结构序列化/反序列化
2. **集成测试**: 使用真实 OCI 环境测试完整流程
3. **Mock 测试**: 使用 `mockito` 模拟 API 响应
4. **错误场景**: 测试各种错误情况的处理

## 后续扩展

1. 实现其他 Compute 操作 (停止、重启、调整大小等)
2. 支持批量操作
3. 实现 VNIC 附加/分离
4. 支持实例配置 (Instance Configuration)
5. 实现实例池 (Instance Pool) 管理
