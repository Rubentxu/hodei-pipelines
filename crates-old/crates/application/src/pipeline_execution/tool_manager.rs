//! Tool Manager Module
//!
//! This module provides tool management capabilities for pipeline execution,
//! including asdf-vm integration, tool caching, and pre-warming functionality.

use async_trait::async_trait;
use hodei_pipelines_domain::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

/// Tool installation error
#[derive(thiserror::Error, Debug)]
pub enum ToolManagerError {
    #[error("asdf-vm not found: {0}")]
    AsdfNotFound(String),

    #[error("Plugin installation failed: {0}")]
    PluginInstallation(String),

    #[error("Tool installation failed: {0}")]
    ToolInstallation(String),

    #[error("Tool verification failed: {0}")]
    ToolVerification(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Tool requirement specification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolRequirement {
    pub name: String,
    pub version: String,
    pub plugin: Option<String>,
}

impl ToolRequirement {
    pub fn new(name: String, version: String, plugin: Option<String>) -> Self {
        Self {
            name,
            version,
            plugin,
        }
    }
}

impl fmt::Display for ToolRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(plugin) = &self.plugin {
            write!(f, "{} {} ({})", self.name, self.version, plugin)
        } else {
            write!(f, "{} {}", self.name, self.version)
        }
    }
}

/// Tool installation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub version: String,
    pub plugin: Option<String>,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub checksum: String,
    pub size_bytes: u64,
}

/// Tool cache entry
#[derive(Debug, Clone)]
pub struct ToolCacheEntry {
    pub metadata: ToolMetadata,
    pub install_path: PathBuf,
    pub download_url: Option<String>,
}

/// Tool Manager with asdf-vm integration
#[derive(Debug)]
pub struct AsdfToolManager {
    workspace_dir: PathBuf,
    cache_dir: PathBuf,
    tools_cache: HashMap<String, ToolCacheEntry>,
    pre_warm_tools: Vec<ToolRequirement>,
}

impl AsdfToolManager {
    /// Create a new ToolManager
    pub fn new(workspace_dir: PathBuf) -> Self {
        let cache_dir = workspace_dir.join(".tool-cache");

        Self {
            workspace_dir,
            cache_dir,
            tools_cache: HashMap::new(),
            pre_warm_tools: vec![
                ToolRequirement::new(
                    "nodejs".to_string(),
                    "18.17.0".to_string(),
                    Some("nodejs".to_string()),
                ),
                ToolRequirement::new(
                    "python".to_string(),
                    "3.11.0".to_string(),
                    Some("python".to_string()),
                ),
                ToolRequirement::new(
                    "rust".to_string(),
                    "1.72.0".to_string(),
                    Some("rust".to_string()),
                ),
            ],
        }
    }

    /// Initialize asdf-vm in workspace
    pub async fn initialize_asdf(&self) -> Result<()> {
        info!(
            workspace_dir = %self.workspace_dir.display(),
            "Initializing asdf-vm"
        );

        // Create .tool-cache directory
        tokio::fs::create_dir_all(&self.cache_dir)
            .await
            .map_err(ToolManagerError::Io)?;

        // Check if asdf is available
        if let Err(_) = tokio::process::Command::new("asdf")
            .arg("--version")
            .output()
            .await
        {
            warn!("asdf-vm not found in PATH, tools may not be available");
        }

        // Initialize asdf directory structure
        let asdf_dir = self.workspace_dir.join(".asdf");
        if !asdf_dir.exists() {
            tokio::fs::create_dir_all(&asdf_dir)
                .await
                .map_err(ToolManagerError::Io)?;
        }

        Ok(())
    }

    /// Pre-warm common tools
    pub async fn pre_warm_tools(&mut self) -> Result<()> {
        info!(
            tools_count = %self.pre_warm_tools.len(),
            "Pre-warming common tools"
        );

        // TODO: Fix borrow checker issue with pre-warm tools
        // for tool in &self.pre_warm_tools {
        //     if !self.is_tool_in_cache(tool).await? {
        //         info!(
        //             tool = %tool,
        //             "Installing pre-warm tool"
        //         );
        //
        //         if let Err(e) = self.install_tool(tool).await {
        //             warn!(
        //                 tool = %tool,
        //                 error = %e,
        //                 "Failed to install pre-warm tool"
        //             );
        //         }
        //     } else {
        //         info!(
        //             tool = %tool,
        //             "Tool already cached, skipping"
        //         );
        //     }
        // }

        Ok(())
    }

    /// Check if tool is already cached
    #[allow(dead_code)]
    async fn is_tool_in_cache(&self, tool: &ToolRequirement) -> Result<bool> {
        let cache_key = self.get_cache_key(tool);
        Ok(self.tools_cache.contains_key(&cache_key))
    }

    /// Install a single tool
    pub async fn install_tool(&mut self, tool: &ToolRequirement) -> Result<()> {
        info!(
            tool = %tool,
            "Installing tool"
        );

        // If tool is already installed, skip
        let cache_key = self.get_cache_key(tool);
        if self.tools_cache.contains_key(&cache_key) {
            info!(
                tool = %tool,
                "Tool already installed, skipping"
            );
            return Ok(());
        }

        // Get plugin name
        let plugin_name = tool.plugin.as_ref().unwrap_or(&tool.name).clone();

        // 1. Install plugin if needed
        self.install_plugin(&plugin_name).await?;

        // 2. Install tool version
        self.install_tool_version(tool, &plugin_name).await?;

        // 3. Verify installation
        self.verify_tool_installation(tool).await?;

        // 4. Cache the tool
        let metadata = ToolMetadata {
            name: tool.name.clone(),
            version: tool.version.clone(),
            plugin: tool.plugin.clone(),
            installed_at: chrono::Utc::now(),
            checksum: "unknown".to_string(), // TODO: implement checksum
            size_bytes: 0,                   // TODO: calculate size
        };

        let install_path = self
            .workspace_dir
            .join(format!(".asdf/installs/{}/{}", plugin_name, tool.version));

        self.tools_cache.insert(
            cache_key,
            ToolCacheEntry {
                metadata,
                install_path,
                download_url: None,
            },
        );

        info!(
            tool = %tool,
            "Tool installed successfully"
        );

        Ok(())
    }

    /// Install asdf plugin
    async fn install_plugin(&self, plugin_name: &str) -> Result<()> {
        info!(plugin = plugin_name, "Installing asdf plugin");

        let output = tokio::process::Command::new("asdf")
            .args(&["plugin", "add", plugin_name])
            .current_dir(&self.workspace_dir)
            .output()
            .await
            .map_err(|e| {
                ToolManagerError::PluginInstallation(format!(
                    "Failed to execute asdf plugin add: {}",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolManagerError::PluginInstallation(format!(
                "asdf plugin add failed: {}",
                stderr
            ))
            .into());
        }

        info!(plugin = plugin_name, "Plugin installed successfully");
        Ok(())
    }

    /// Install specific tool version
    async fn install_tool_version(&self, tool: &ToolRequirement, plugin_name: &str) -> Result<()> {
        info!(
            plugin = plugin_name,
            version = %tool.version,
            "Installing tool version"
        );

        let output = tokio::process::Command::new("asdf")
            .args(&["plugin", "update", plugin_name, "--stdin", &tool.version])
            .current_dir(&self.workspace_dir)
            .output()
            .await
            .map_err(|e| {
                ToolManagerError::ToolInstallation(format!(
                    "Failed to execute asdf plugin update: {}",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolManagerError::ToolInstallation(format!(
                "asdf plugin update failed: {}",
                stderr
            ))
            .into());
        }

        Ok(())
    }

    /// Verify tool installation
    async fn verify_tool_installation(&self, tool: &ToolRequirement) -> Result<()> {
        let plugin_name = tool.plugin.as_ref().unwrap_or(&tool.name).clone();

        let output = tokio::process::Command::new("asdf")
            .args(&["which", &plugin_name])
            .current_dir(&self.workspace_dir)
            .output()
            .await
            .map_err(|e| {
                ToolManagerError::ToolVerification(format!("Failed to execute asdf which: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolManagerError::ToolVerification(format!(
                "Tool verification failed: {}",
                stderr
            ))
            .into());
        }

        Ok(())
    }

    /// Get cache key for a tool
    fn get_cache_key(&self, tool: &ToolRequirement) -> String {
        format!("{}:{}", tool.name, tool.version)
    }

    /// Get path to installed tool
    pub fn get_tool_path(&self, tool: &ToolRequirement) -> Option<PathBuf> {
        let cache_key = self.get_cache_key(tool);
        self.tools_cache
            .get(&cache_key)
            .map(|entry| entry.install_path.clone())
    }

    /// List all cached tools
    pub fn list_cached_tools(&self) -> Vec<ToolMetadata> {
        self.tools_cache
            .values()
            .map(|entry| entry.metadata.clone())
            .collect()
    }

    /// Set environment variables for asdf
    pub fn set_asdf_env(&self) -> HashMap<String, String> {
        let mut env_vars = HashMap::new();

        // Set ASDF_DIR
        env_vars.insert(
            "ASDF_DIR".to_string(),
            self.workspace_dir
                .join(".asdf")
                .to_string_lossy()
                .to_string(),
        );

        // Set ASDF_DATA_DIR
        env_vars.insert(
            "ASDF_DATA_DIR".to_string(),
            self.workspace_dir
                .join(".asdf")
                .to_string_lossy()
                .to_string(),
        );

        // Add asdf shims to PATH
        let shims_dir = self
            .workspace_dir
            .join(".asdf")
            .join("shims")
            .to_string_lossy()
            .to_string();

        env_vars.insert("PATH".to_string(), shims_dir);

        env_vars
    }
}

#[async_trait]
impl super::pipeline_step_executor::ToolManager for AsdfToolManager {
    async fn install_tools(
        &self,
        _workspace_path: &Path,
        tools: &[super::pipeline_step_executor::ToolRequirement],
    ) -> hodei_pipelines_domain::Result<()> {
        // Initialize asdf in workspace
        self.initialize_asdf().await?;

        // Install each tool
        for tool in tools {
            let _asdf_tool = ToolRequirement {
                name: tool.name.clone(),
                version: tool.version.clone(),
                plugin: tool.plugin.clone(),
            };

            // This would need to be mutable, so we'd need to rethink the API
            // For now, return an error indicating this needs to be called on a mutable reference
            return Err(
                super::pipeline_step_executor::ExecutionError::ToolInstallation(
                    "Tool installation requires mutable reference".to_string(),
                )
                .into(),
            );
        }

        Ok(())
    }

    async fn is_tool_installed(
        &self,
        _workspace_path: &Path,
        tool: &super::pipeline_step_executor::ToolRequirement,
    ) -> hodei_pipelines_domain::Result<bool> {
        let cache_key = format!("{}:{}", tool.name, tool.version);
        Ok(self.tools_cache.contains_key(&cache_key))
    }
}

impl From<ToolManagerError> for hodei_pipelines_domain::DomainError {
    fn from(error: ToolManagerError) -> Self {
        hodei_pipelines_domain::DomainError::Infrastructure(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{AsdfToolManager, ToolCacheEntry, ToolMetadata, ToolRequirement};
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_tool_requirement_display() {
        let tool = ToolRequirement::new(
            "nodejs".to_string(),
            "18.0.0".to_string(),
            Some("nodejs".to_string()),
        );
        assert_eq!(tool.to_string(), "nodejs 18.0.0 (nodejs)");
    }

    #[tokio::test]
    async fn test_tool_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let manager = AsdfToolManager::new(workspace);
        assert_eq!(manager.pre_warm_tools.len(), 3);
    }

    #[tokio::test]
    async fn test_get_tool_path() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut manager = AsdfToolManager::new(workspace);

        let tool = ToolRequirement::new(
            "nodejs".to_string(),
            "18.0.0".to_string(),
            Some("nodejs".to_string()),
        );

        // Tool not cached yet
        assert_eq!(manager.get_tool_path(&tool), None);

        // Add to cache
        let metadata = ToolMetadata {
            name: tool.name.clone(),
            version: tool.version.clone(),
            plugin: tool.plugin.clone(),
            installed_at: chrono::Utc::now(),
            checksum: "test".to_string(),
            size_bytes: 100,
        };

        let install_path = PathBuf::from("/test/install/path");
        manager.tools_cache.insert(
            "nodejs:18.0.0".to_string(),
            ToolCacheEntry {
                metadata,
                install_path: install_path.clone(),
                download_url: None,
            },
        );

        assert_eq!(manager.get_tool_path(&tool), Some(install_path));
    }

    #[test]
    fn test_set_asdf_env() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let manager = AsdfToolManager::new(workspace);
        let env_vars = manager.set_asdf_env();

        assert!(env_vars.contains_key("ASDF_DIR"));
        assert!(env_vars.contains_key("ASDF_DATA_DIR"));
        assert!(env_vars.contains_key("PATH"));
    }
}
