//! Configuration de zer0build.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration globale de zer0build.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Répertoire d'installation par défaut des modules.
    pub install_dir: Option<PathBuf>,
    
    /// Clé API OpenRouter par défaut.
    pub openrouter_api_key: Option<String>,
    
    /// Modèle OpenRouter par défaut.
    pub default_model: String,
    
    /// Cibles de compilation par défaut.
    pub default_targets: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            install_dir: None,
            openrouter_api_key: None,
            default_model: "anthropic/claude-3.5-sonnet".to_string(),
            default_targets: vec!["x86_64-unknown-linux-gnu".to_string()],
        }
    }
}

impl Config {
    /// Charge la configuration depuis le fichier de config.
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }
    
    /// Sauvegarde la configuration.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        std::fs::create_dir_all(path.parent().unwrap())?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
    
    /// Chemin du fichier de configuration.
    fn config_path() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Public\\AppData\\Roaming".to_string())))
                .join("zer0build")
                .join("config.toml")
        }
        #[cfg(not(target_os = "windows"))]
        {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("~/.config"))
                .join("zer0build")
                .join("config.toml")
        }
    }
}
