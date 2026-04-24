use super::models::{Instance, LaunchInstanceDetails};
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
    
    /// Launch a new compute instance
    pub async fn launch_instance(
        &self,
        details: &LaunchInstanceDetails,
    ) -> Result<Instance, Box<dyn std::error::Error>> {
        let url = format!("{}/20160918/instances", self.endpoint());
        let body = serde_json::to_vec(details)?;
        
        // Sign and send request
        let auth_header = self.signer.sign_request(
            "POST",
            "/20160918/instances",
            &format!("iaas.{}.oraclecloud.com", self.region),
            Some(&body),
            &[("content-type", "application/json")],
        )?;
        
        let date_header = RequestSigner::get_date_header();
        
        let response = self.http_client
            .post(&url)
            .header("authorization", auth_header)
            .header("date", date_header)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(format!("API error {}: {}", status, error_text).into());
        }
        
        let instance: Instance = response.json().await?;
        Ok(instance)
    }
    
    /// Get instance details
    pub async fn get_instance(
        &self,
        instance_id: &str,
    ) -> Result<Instance, Box<dyn std::error::Error>> {
        let path = format!("/20160918/instances/{}", instance_id);
        let url = format!("{}{}", self.endpoint(), path);
        
        let auth_header = self.signer.sign_request(
            "GET",
            &path,
            &format!("iaas.{}.oraclecloud.com", self.region),
            None,
            &[],
        )?;
        
        let date_header = RequestSigner::get_date_header();
        
        let response = self.http_client
            .get(&url)
            .header("authorization", auth_header)
            .header("date", date_header)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(format!("API error {}: {}", status, error_text).into());
        }
        
        let instance: Instance = response.json().await?;
        Ok(instance)
    }
    
    /// Terminate an instance
    pub async fn terminate_instance(
        &self,
        instance_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = format!("/20160918/instances/{}", instance_id);
        let url = format!("{}{}", self.endpoint(), path);
        
        let auth_header = self.signer.sign_request(
            "DELETE",
            &path,
            &format!("iaas.{}.oraclecloud.com", self.region),
            None,
            &[],
        )?;
        
        let date_header = RequestSigner::get_date_header();
        
        let response = self.http_client
            .delete(&url)
            .header("authorization", auth_header)
            .header("date", date_header)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(format!("API error {}: {}", status, error_text).into());
        }
        
        Ok(())
    }
}
