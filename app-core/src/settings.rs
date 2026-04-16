use crate::types::AppSettings;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl Default for SettingsStore {
    fn default() -> Self {
        let mut base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.push("barrier-rust");
        base.push("settings.toml");
        Self { path: base }
    }
}

impl SettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<AppSettings, String> {
        if !self.path.exists() {
            return Ok(AppSettings::default());
        }

        let content =
            fs::read_to_string(&self.path).map_err(|e| format!("读取配置文件失败: {}", e))?;
        toml::from_str::<AppSettings>(&content).map_err(|e| format!("解析配置文件失败: {}", e))
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {}", e))?;
        }
        let payload = toml::to_string_pretty(settings).map_err(|e| format!("序列化配置失败: {}", e))?;
        fs::write(&self.path, payload).map_err(|e| format!("写入配置文件失败: {}", e))
    }
}
