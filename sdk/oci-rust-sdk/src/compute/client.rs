use super::models::{AvailabilityDomain, Image, Instance, LaunchInstanceDetails, Shape, Subnet, Vcn};
use crate::auth::{ConfigurationProvider, RequestSigner};
use reqwest::Client;

pub struct ComputeClient {
    http_client: Client,
    signer: RequestSigner,
    region: String,
}

impl ComputeClient {
    /// Create a new compute client
    pub fn new(config: &dyn ConfigurationProvider) -> Result<Self, Box<dyn std::error::Error>> {
        let region = config.region()?;
        let signer = RequestSigner::new(config)?;
        
        Ok(Self {
            http_client: Client::new(),
            signer,
            region,
        })
    }
    
    /// Get the base endpoint for compute service
    fn endpoint(&self) -> String {
        format!("https://iaas.{}.oraclecloud.com", self.region)
    }
    
    /// Get the host for API requests
    fn host(&self) -> String {
        format!("iaas.{}.oraclecloud.com", self.region)
    }
    
    /// Handle API response errors
    async fn handle_response<T: serde::de::DeserializeOwned>(
        response: reqwest::Response,
    ) -> Result<T, Box<dyn std::error::Error>> {
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(format!("API error {}: {}", status, error_text).into());
        }
        Ok(response.json().await?)
    }
    
    /// Build a signed request with additional headers
    fn build_signed_request(
        &self,
        method: &str,
        url: &str,
        path: &str,
        body: Option<&[u8]>,
        headers: &[(&str, &str)],
    ) -> Result<reqwest::RequestBuilder, Box<dyn std::error::Error>> {
        let (auth_header, additional_headers) = self.signer.sign_request(
            method,
            path,
            &self.host(),
            body,
            headers,
        )?;
        
        let mut request = match method {
            "GET" => self.http_client.get(url),
            "POST" => self.http_client.post(url),
            "DELETE" => self.http_client.delete(url),
            _ => return Err(format!("Unsupported HTTP method: {}", method).into()),
        };
        
        request = request.header("authorization", auth_header);
        
        for (key, value) in additional_headers {
            request = request.header(key, value);
        }
        
        Ok(request)
    }
    
    /// Launch a new compute instance
    pub async fn launch_instance(
        &self,
        details: &LaunchInstanceDetails,
    ) -> Result<Instance, Box<dyn std::error::Error>> {
        let path = "/20160918/instances";
        let url = format!("{}{}", self.endpoint(), path);
        let body = serde_json::to_vec(details)?;
        
        let response = self.build_signed_request(
            "POST",
            &url,
            path,
            Some(&body),
            &[("content-type", "application/json")],
        )?
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await?;
        
        Self::handle_response(response).await
    }
    
    /// Get instance details
    pub async fn get_instance(
        &self,
        instance_id: &str,
    ) -> Result<Instance, Box<dyn std::error::Error>> {
        let path = format!("/20160918/instances/{}", instance_id);
        let url = format!("{}{}", self.endpoint(), path);
        
        let response = self.build_signed_request("GET", &url, &path, None, &[])?
            .send()
            .await?;
        
        Self::handle_response(response).await
    }
    
    /// Terminate an instance
    pub async fn terminate_instance(
        &self,
        instance_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = format!("/20160918/instances/{}", instance_id);
        let url = format!("{}{}", self.endpoint(), path);
        
        let response = self.build_signed_request("DELETE", &url, &path, None, &[])?
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(format!("API error {}: {}", status, error_text).into());
        }
        
        Ok(())
    }

    /// List availability domains in a compartment
    pub async fn list_availability_domains(
        &self,
        compartment_id: &str,
    ) -> Result<Vec<AvailabilityDomain>, Box<dyn std::error::Error>> {
        let path = format!("/20160918/availabilityDomains?compartmentId={}", compartment_id);
        let url = format!("https://identity.{}.oraclecloud.com{}", self.region, path);
        let host = format!("identity.{}.oraclecloud.com", self.region);

        let (auth_header, additional_headers) = self.signer.sign_request("GET", &path, &host, None, &[])?;

        let mut request = self.http_client.get(&url).header("authorization", auth_header);
        
        for (key, value) in additional_headers {
            request = request.header(key, value);
        }
        
        let response = request.send().await?;

        Self::handle_response(response).await
    }

    /// List images in a compartment
    pub async fn list_images(
        &self,
        compartment_id: &str,
    ) -> Result<Vec<Image>, Box<dyn std::error::Error>> {
        self.list_images_filtered(compartment_id, None, None).await
    }
    
    /// List images in a compartment with optional filters
    pub async fn list_images_filtered(
        &self,
        compartment_id: &str,
        operating_system: Option<&str>,
        operating_system_version: Option<&str>,
    ) -> Result<Vec<Image>, Box<dyn std::error::Error>> {
        let mut query_params = vec![format!("compartmentId={}", compartment_id)];
        
        if let Some(os) = operating_system {
            query_params.push(format!("operatingSystem={}", urlencoding::encode(os)));
        }
        
        if let Some(version) = operating_system_version {
            query_params.push(format!("operatingSystemVersion={}", urlencoding::encode(version)));
        }
        
        let query_string = query_params.join("&");
        let path = format!("/20160918/images?{}", query_string);
        let url = format!("{}{}", self.endpoint(), path);

        let response = self.build_signed_request("GET", &url, &path, None, &[])?
            .send()
            .await?;

        Self::handle_response(response).await
    }

    /// List shapes available in a compartment
    pub async fn list_shapes(
        &self,
        compartment_id: &str,
    ) -> Result<Vec<Shape>, Box<dyn std::error::Error>> {
        let path = format!("/20160918/shapes?compartmentId={}", compartment_id);
        let url = format!("{}{}", self.endpoint(), path);

        let response = self.build_signed_request("GET", &url, &path, None, &[])?
            .send()
            .await?;

        Self::handle_response(response).await
    }

    /// List VCNs in a compartment
    pub async fn list_vcns(
        &self,
        compartment_id: &str,
    ) -> Result<Vec<Vcn>, Box<dyn std::error::Error>> {
        let path = format!("/20160918/vcns?compartmentId={}", compartment_id);
        let url = format!("{}{}", self.endpoint(), path);

        let response = self.build_signed_request("GET", &url, &path, None, &[])?
            .send()
            .await?;

        Self::handle_response(response).await
    }

    /// List subnets in a compartment
    pub async fn list_subnets(
        &self,
        compartment_id: &str,
    ) -> Result<Vec<Subnet>, Box<dyn std::error::Error>> {
        let path = format!("/20160918/subnets?compartmentId={}", compartment_id);
        let url = format!("{}{}", self.endpoint(), path);

        let response = self.build_signed_request("GET", &url, &path, None, &[])?
            .send()
            .await?;

        Self::handle_response(response).await
    }
}
