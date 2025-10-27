use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub profiles: HashMap<String, Profile>,
    pub auto_switch: AutoSwitch,
    pub monitor_detection: MonitorDetection,
    pub current_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub monitors: Vec<String>,
    pub wallpaper_dirs: Vec<PathBuf>,
    pub transition: String,
    pub transition_duration: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSwitch {
    pub enabled: bool,
    pub interval: u64,
    pub mode: SwitchMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SwitchMode {
    Random,
    Sequential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorDetection {
    pub enabled: bool,
}

impl Config {
    pub fn default_path() -> Option<String> {
        dirs::config_dir().map(|p| {
            p.join("swww-manager/config.toml")
                .to_string_lossy()
                .to_string()
        })
    }

    pub fn load(path: Option<&str>) -> Result<Self> {
        let path = path
            .map(PathBuf::from)
            .or_else(|| dirs::config_dir().map(|p| p.join("swww-manager/config.toml")))
            .context("Could not determine config path")?;

        if !path.exists() {
            info!("Config file not found, using defaults");
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {:?}", path))?;
        
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config: {:?}", path))
    }

    pub fn save(&self, path: Option<&Path>) -> Result<()> {
        let path = path
            .map(PathBuf::from)
            .or_else(|| dirs::config_dir().map(|p| p.join("swww-manager/config.toml")))
            .context("Could not determine config path")?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(&path, content)?;
        info!("Config saved to {:?}", path);
        Ok(())
    }

    pub fn default() -> Self {
        let mut profiles = HashMap::new();
        
        // Default profile
        profiles.insert(
            "default".to_string(),
            Profile {
                monitors: vec!["*".to_string()],
                wallpaper_dirs: vec![
                    dirs::home_dir()
                        .unwrap_or_default()
                        .join("Pictures/Wallpapers")
                ],
                transition: "wipe".to_string(),
                transition_duration: 2,
            },
        );

        // Example dual monitor profile
        profiles.insert(
            "dual_monitor".to_string(),
            Profile {
                monitors: vec!["DP-1".to_string(), "HDMI-A-1".to_string()],
                wallpaper_dirs: vec![
                    dirs::home_dir()
                        .unwrap_or_default()
                        .join("Pictures/Wallpapers/Dual")
                ],
                transition: "fade".to_string(),
                transition_duration: 3,
            },
        );

        // Example laptop profile
        profiles.insert(
            "laptop".to_string(),
            Profile {
                monitors: vec!["eDP-1".to_string()],
                wallpaper_dirs: vec![
                    dirs::home_dir()
                        .unwrap_or_default()
                        .join("Pictures/Wallpapers/Laptop")
                ],
                transition: "simple".to_string(),
                transition_duration: 1,
            },
        );

        Self {
            profiles,
            auto_switch: AutoSwitch {
                enabled: false,
                interval: 300,
                mode: SwitchMode::Random,
            },
            monitor_detection: MonitorDetection { enabled: true },
            current_profile: "default".to_string(),
        }
    }

    pub fn generate_example() -> Result<()> {
        let config = Self::default();
        let path = dirs::config_dir()
            .map(|p| p.join("swww-manager/config.toml"))
            .context("Could not determine config path")?;
        
        config.save(Some(&path))?;
        
        println!("\nExample configuration:");
        println!("{}", toml::to_string_pretty(&config)?);
        
        Ok(())
    }
}
