pub mod config;
pub mod monitor;
pub mod wallpaper;
pub mod profile;
pub mod protocol;
pub mod hyprland_events;
pub mod hyprland_ipc;
pub mod notify;

pub use config::Config;
pub use monitor::MonitorManager;
pub use wallpaper::WallpaperManager;
pub use profile::ProfileManager;
pub use hyprland_ipc::HyprlandIPC;

use anyhow::Result;

pub struct Manager {
    config: Config,
    monitor_manager: MonitorManager,
    wallpaper_manager: WallpaperManager,
    profile_manager: ProfileManager,
}

impl Manager {
    pub fn new(config: Config) -> Self {
        Self {
            monitor_manager: MonitorManager::new(),
            wallpaper_manager: WallpaperManager::new(),
            profile_manager: ProfileManager::new(config.clone()),
            config,
        }
    }

    pub async fn switch_wallpaper(&mut self) -> Result<String> {
        let profile = self.profile_manager.current_profile()?;
        let wallpaper = self.wallpaper_manager.get_wallpaper(profile, &self.config)?;
        self.wallpaper_manager.set_wallpaper(&wallpaper, profile).await?;
        notify::send("Wallpaper switched", &wallpaper).await?;
        Ok(wallpaper)
    }

    pub async fn switch_profile(&mut self, name: &str) -> Result<()> {
        self.profile_manager.switch_to(name)?;
        self.config.current_profile = name.to_string();
        self.config.save(None)?;
        notify::send("Profile switched", name).await?;
        self.switch_wallpaper().await?;
        Ok(())
    }

    pub fn list_profiles(&self) {
        self.profile_manager.list();
    }

    pub async fn detect_and_switch(&mut self) -> Result<Option<String>> {
        let monitors = self.monitor_manager.get_monitors().await?;
        
        if let Some(profile_name) = self.profile_manager.detect_profile(&monitors)? {
            if profile_name != self.config.current_profile {
                self.switch_profile(&profile_name).await?;
                return Ok(Some(profile_name));
            }
        }
        
        Ok(None)
    }
}
