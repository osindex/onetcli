//! 应用配置模块
//!
//! 通过 `build.rs` 在编译时将环境变量内嵌到二进制文件中，
//! 运行时也可通过同名环境变量覆盖，也可以从用户配置目录读取。
//!
//! # 配置优先级
//! 1. 运行时环境变量（最高优先级，用于开发调试）
//! 2. 用户配置文件（`$CONFIG/one-hub/settings.json`）
//! 3. 当前启动目录下的旧配置文件（`./one-hub/supabase.json`，兼容旧版本）
//! 4. 用户配置目录下的旧配置文件（`$CONFIG/one-hub/supabase.json`，兼容旧版本）
//! 5. 编译时环境变量（发布版本内置，由 build.rs 通过 cargo:rustc-env 注入）
//!
//! # 发版构建
//! ```bash
//! SUPABASE_URL=https://xxx.supabase.co \
//! SUPABASE_ANON_KEY=eyJ... \
//! cargo build --release
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::storage::manager::get_config_dir;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppSettingsConfigFile {
    #[serde(default)]
    supabase_url: String,
    #[serde(default)]
    supabase_anon_key: String,
    #[serde(default)]
    update_url: String,
    #[serde(default)]
    update_download_url: String,
}

impl AppSettingsConfigFile {
    fn config_path() -> Option<PathBuf> {
        get_config_dir().ok().map(|dir| dir.join("settings.json"))
    }

    fn load() -> Option<Self> {
        let path = Self::config_path()?;
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }
}

/// Supabase 配置
#[derive(Debug, Clone)]
pub struct SupabaseConfig {
    /// 项目 URL
    pub project_url: String,
    /// API Key (anon key)
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SupabaseConfigFile {
    project_url: String,
    api_key: String,
}

/// 应用更新配置
#[derive(Debug, Clone)]
pub struct UpdateConfig {
    /// 版本检查接口地址
    pub update_url: String,
    /// 更新下载页地址（可选）
    pub download_url: Option<String>,
}

impl UpdateConfig {
    /// 获取更新配置
    ///
    /// 优先级：运行时环境变量 > 编译时环境变量
    pub fn get() -> Self {
        if let Some(file_config) = Self::from_settings_file() {
            return file_config;
        }

        Self {
            update_url: Self::get_update_url(),
            download_url: Self::get_download_url(),
        }
    }

    fn from_settings_file() -> Option<Self> {
        let settings = AppSettingsConfigFile::load()?;
        let update_url = settings.update_url.trim().to_string();
        let download_url = settings.update_download_url.trim().to_string();

        if update_url.is_empty() {
            return None;
        }

        Some(Self {
            update_url,
            download_url: if download_url.is_empty() {
                None
            } else {
                Some(download_url)
            },
        })
    }

    /// 获取更新接口地址
    fn get_update_url() -> String {
        if let Ok(url) = std::env::var("ONETCLI_UPDATE_URL") {
            if !url.trim().is_empty() {
                return url;
            }
        }

        option_env!("ONETCLI_UPDATE_URL")
            .unwrap_or_default()
            .to_string()
    }

    /// 获取下载页地址
    fn get_download_url() -> Option<String> {
        if let Ok(url) = std::env::var("ONETCLI_UPDATE_DOWNLOAD_URL") {
            if !url.trim().is_empty() {
                return Some(url);
            }
        }

        option_env!("ONETCLI_UPDATE_DOWNLOAD_URL").and_then(|value| {
            if value.trim().is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
    }

    /// 检查配置是否有效
    pub fn is_valid(&self) -> bool {
        !self.update_url.trim().is_empty()
    }
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self::get()
    }
}

impl SupabaseConfig {
    /// 获取 Supabase 配置
    ///
    /// 优先级：运行时环境变量 > 当前启动目录配置文件 > 用户配置目录配置文件 > 编译时环境变量
    pub fn get() -> Self {
        if let Some(settings) = AppSettingsConfigFile::load() {
            let project_url = settings.supabase_url.trim().to_string();
            let api_key = settings.supabase_anon_key.trim().to_string();
            if !project_url.is_empty() && !api_key.is_empty() {
                return Self { project_url, api_key };
            }
        }

        if let Some(config) = Self::from_file_candidates() {
            return config;
        }

        Self {
            project_url: Self::get_url(),
            api_key: Self::get_api_key(),
        }
    }

    fn from_file_candidates() -> Option<Self> {
        for path in Self::config_paths() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(file_config) = serde_json::from_str::<SupabaseConfigFile>(&content) {
                    let project_url = file_config.project_url.trim().to_string();
                    let api_key = file_config.api_key.trim().to_string();

                    if !project_url.is_empty() && !api_key.is_empty() {
                        return Some(Self { project_url, api_key });
                    }
                }
            }
        }

        None
    }

    fn config_paths() -> Vec<std::path::PathBuf> {
        let mut paths = Vec::new();

        if let Ok(cwd) = std::env::current_dir() {
            paths.push(cwd.join("one-hub").join("supabase.json"));
        }

        if let Some(config_dir) = dirs::config_dir() {
            paths.push(config_dir.join("one-hub").join("supabase.json"));
        }

        paths
    }

    /// 获取项目 URL
    fn get_url() -> String {
        if let Ok(url) = std::env::var("SUPABASE_URL") {
            if !url.trim().is_empty() {
                return url;
            }
        }

        // 编译时环境变量
        option_env!("SUPABASE_URL").unwrap_or_default().to_string()
    }

    /// 获取 API Key
    fn get_api_key() -> String {
        if let Ok(key) = std::env::var("SUPABASE_ANON_KEY") {
            if !key.trim().is_empty() {
                return key;
            }
        }

        // 编译时环境变量
        option_env!("SUPABASE_ANON_KEY")
            .unwrap_or_default()
            .to_string()
    }

    /// 检查配置是否有效
    pub fn is_valid(&self) -> bool {
        !self.project_url.is_empty() && !self.api_key.is_empty()
    }
}

impl Default for SupabaseConfig {
    fn default() -> Self {
        Self::get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_get() {
        let config = SupabaseConfig::get();
        // 测试环境可能没有配置，只验证不会 panic
        let _ = config.is_valid();
    }

    #[test]
    fn test_update_config_get() {
        let config = UpdateConfig::get();
        let _ = config.is_valid();
    }
}
