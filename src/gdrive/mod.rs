/*!
Google Drive API integration for Dattavani ASR

Provides basic integration with Google Drive API for file access and streaming.
Uses direct HTTP requests to Google Drive API instead of SDK dependencies.
*/

use serde::{Deserialize, Serialize};
use reqwest::Client;
use regex::Regex;
use tracing::warn;

use crate::config::Config;
use crate::error::{DattavaniError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size: Option<u64>,
    pub created_time: String,
    pub modified_time: String,
    pub parents: Option<Vec<String>>,
    pub web_view_link: Option<String>,
    pub web_content_link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub display_name: String,
    pub email_address: String,
    pub photo_link: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GDriveClient {
    client: Client,
    config: Config,
    access_token: Option<String>,
}

impl GDriveClient {
    pub async fn new(config: Config) -> Result<Self> {
        let client = Client::new();
        
        // For now, we'll implement a basic version without full OAuth
        // In a production environment, you would implement proper OAuth flow
        let access_token = Self::get_access_token_from_env().await?;
        
        Ok(Self {
            client,
            config,
            access_token,
        })
    }
    
    async fn get_access_token_from_env() -> Result<Option<String>> {
        // Try to get access token from environment variable
        // In production, this would use proper OAuth flow or service account
        if let Ok(token) = std::env::var("GOOGLE_ACCESS_TOKEN") {
            Ok(Some(token))
        } else {
            warn!("No Google access token found. Set GOOGLE_ACCESS_TOKEN environment variable.");
            Ok(None)
        }
    }
    
    pub async fn get_user_info(&self) -> Result<UserInfo> {
        let access_token = self.access_token.as_ref()
            .ok_or_else(|| DattavaniError::authentication("No access token available"))?;
        
        let url = "https://www.googleapis.com/drive/v3/about?fields=user";
        
        let response = self.client
            .get(url)
            .bearer_auth(access_token)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(DattavaniError::google_drive(format!("API error: {}", error_text)));
        }
        
        let about_response: serde_json::Value = response.json().await?;
        let user = about_response["user"].as_object()
            .ok_or_else(|| DattavaniError::google_drive("Invalid user info response"))?;
        
        Ok(UserInfo {
            display_name: user["displayName"].as_str().unwrap_or("Unknown").to_string(),
            email_address: user["emailAddress"].as_str().unwrap_or("Unknown").to_string(),
            photo_link: user["photoLink"].as_str().map(|s| s.to_string()),
        })
    }
    
    pub async fn get_file_info(&self, file_id: &str) -> Result<DriveFile> {
        let access_token = self.access_token.as_ref()
            .ok_or_else(|| DattavaniError::authentication("No access token available"))?;
        
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?fields=id,name,mimeType,size,createdTime,modifiedTime,parents,webViewLink,webContentLink",
            file_id
        );
        
        let response = self.client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(DattavaniError::google_drive(format!("Failed to get file info: {}", error_text)));
        }
        
        let file_data: serde_json::Value = response.json().await?;
        
        Ok(DriveFile {
            id: file_data["id"].as_str().unwrap_or_default().to_string(),
            name: file_data["name"].as_str().unwrap_or_default().to_string(),
            mime_type: file_data["mimeType"].as_str().unwrap_or_default().to_string(),
            size: file_data["size"].as_str().and_then(|s| s.parse().ok()),
            created_time: file_data["createdTime"].as_str().unwrap_or_default().to_string(),
            modified_time: file_data["modifiedTime"].as_str().unwrap_or_default().to_string(),
            parents: file_data["parents"].as_array().map(|arr| {
                arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
            }),
            web_view_link: file_data["webViewLink"].as_str().map(|s| s.to_string()),
            web_content_link: file_data["webContentLink"].as_str().map(|s| s.to_string()),
        })
    }
    
    pub async fn list_files_in_folder(&self, folder_id: &str, pattern: Option<&str>) -> Result<Vec<DriveFile>> {
        let access_token = self.access_token.as_ref()
            .ok_or_else(|| DattavaniError::authentication("No access token available"))?;
        
        let mut query = format!("'{}' in parents and trashed=false", folder_id);
        
        // Add pattern filter if provided
        if let Some(pattern) = pattern {
            // Convert glob pattern to Google Drive query
            let name_filter = pattern.replace("*", "");
            if !name_filter.is_empty() {
                query.push_str(&format!(" and name contains '{}'", name_filter));
            }
        }
        
        let url = format!(
            "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name,mimeType,size,createdTime,modifiedTime,parents,webViewLink,webContentLink)",
            urlencoding::encode(&query)
        );
        
        let response = self.client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(DattavaniError::google_drive(format!("Failed to list files: {}", error_text)));
        }
        
        let list_response: serde_json::Value = response.json().await?;
        let files = list_response["files"].as_array()
            .ok_or_else(|| DattavaniError::google_drive("Invalid file list response"))?;
        
        let mut result = Vec::new();
        for file_data in files {
            result.push(DriveFile {
                id: file_data["id"].as_str().unwrap_or_default().to_string(),
                name: file_data["name"].as_str().unwrap_or_default().to_string(),
                mime_type: file_data["mimeType"].as_str().unwrap_or_default().to_string(),
                size: file_data["size"].as_str().and_then(|s| s.parse().ok()),
                created_time: file_data["createdTime"].as_str().unwrap_or_default().to_string(),
                modified_time: file_data["modifiedTime"].as_str().unwrap_or_default().to_string(),
                parents: file_data["parents"].as_array().map(|arr| {
                    arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
                }),
                web_view_link: file_data["webViewLink"].as_str().map(|s| s.to_string()),
                web_content_link: file_data["webContentLink"].as_str().map(|s| s.to_string()),
            });
        }
        
        Ok(result)
    }
    
    pub async fn get_download_stream(&self, file_id: &str) -> Result<reqwest::Response> {
        let access_token = self.access_token.as_ref()
            .ok_or_else(|| DattavaniError::authentication("No access token available"))?;
        
        let url = format!("https://www.googleapis.com/drive/v3/files/{}?alt=media", file_id);
        
        let response = self.client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(DattavaniError::google_drive(format!("Failed to get download stream: {}", error_text)));
        }
        
        Ok(response)
    }
    
    pub async fn get_partial_content(&self, file_id: &str, start: u64, end: Option<u64>) -> Result<bytes::Bytes> {
        let access_token = self.access_token.as_ref()
            .ok_or_else(|| DattavaniError::authentication("No access token available"))?;
        
        let url = format!("https://www.googleapis.com/drive/v3/files/{}?alt=media", file_id);
        
        let range_header = if let Some(end) = end {
            format!("bytes={}-{}", start, end)
        } else {
            format!("bytes={}-", start)
        };
        
        let response = self.client
            .get(&url)
            .bearer_auth(access_token)
            .header("Range", range_header)
            .send()
            .await?;
        
        if !response.status().is_success() && response.status().as_u16() != 206 {
            let error_text = response.text().await.unwrap_or_default();
            return Err(DattavaniError::google_drive(format!("Failed to get partial content: {}", error_text)));
        }
        
        let bytes = response.bytes().await?;
        Ok(bytes)
    }
    
    pub fn extract_file_id_from_url(url: &str) -> Result<String> {
        let patterns = [
            r"drive\.google\.com/file/d/([a-zA-Z0-9_-]+)",
            r"drive\.google\.com/open\?id=([a-zA-Z0-9_-]+)",
            r"docs\.google\.com/.*?/d/([a-zA-Z0-9_-]+)",
        ];
        
        for pattern in &patterns {
            let regex = Regex::new(pattern)?;
            if let Some(captures) = regex.captures(url) {
                if let Some(file_id) = captures.get(1) {
                    return Ok(file_id.as_str().to_string());
                }
            }
        }
        
        Err(DattavaniError::validation(format!("Could not extract file ID from URL: {}", url)))
    }
    
    pub fn is_google_drive_url(url: &str) -> bool {
        url.contains("drive.google.com") || url.contains("docs.google.com")
    }
    
    pub async fn create_folder(&self, name: &str, parent_id: Option<&str>) -> Result<String> {
        let access_token = self.access_token.as_ref()
            .ok_or_else(|| DattavaniError::authentication("No access token available"))?;
        
        let mut metadata = serde_json::json!({
            "name": name,
            "mimeType": "application/vnd.google-apps.folder"
        });
        
        if let Some(parent) = parent_id {
            metadata["parents"] = serde_json::json!([parent]);
        }
        
        let response = self.client
            .post("https://www.googleapis.com/drive/v3/files")
            .bearer_auth(access_token)
            .json(&metadata)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(DattavaniError::google_drive(format!("Failed to create folder: {}", error_text)));
        }
        
        let folder_data: serde_json::Value = response.json().await?;
        let folder_id = folder_data["id"].as_str()
            .ok_or_else(|| DattavaniError::google_drive("Invalid folder creation response"))?;
        
        Ok(folder_id.to_string())
    }
    
    pub async fn upload_file(&self, name: &str, content: &[u8], parent_id: Option<&str>, mime_type: Option<&str>) -> Result<String> {
        let access_token = self.access_token.as_ref()
            .ok_or_else(|| DattavaniError::authentication("No access token available"))?;
        
        // First, create the file metadata
        let mut metadata = serde_json::json!({
            "name": name
        });
        
        if let Some(parent) = parent_id {
            metadata["parents"] = serde_json::json!([parent]);
        }
        
        if let Some(mime) = mime_type {
            metadata["mimeType"] = serde_json::json!(mime);
        }
        
        // Use multipart upload
        let form = reqwest::multipart::Form::new()
            .text("metadata", metadata.to_string())
            .part("media", reqwest::multipart::Part::bytes(content.to_vec()));
        
        let response = self.client
            .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart")
            .bearer_auth(access_token)
            .multipart(form)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(DattavaniError::google_drive(format!("Failed to upload file: {}", error_text)));
        }
        
        let file_data: serde_json::Value = response.json().await?;
        let file_id = file_data["id"].as_str()
            .ok_or_else(|| DattavaniError::google_drive("Invalid file upload response"))?;
        
        Ok(file_id.to_string())
    }
}
