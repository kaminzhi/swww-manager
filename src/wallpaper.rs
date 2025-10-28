use crate::config::{Config, Profile, SwitchMode};
use anyhow::{Context, Result};
use glob::glob;
use rand::seq::SliceRandom;
use std::path::PathBuf;
use std::process::Command;
use tracing::info;
use rand::prelude::IndexedMutRandom;

#[derive(Clone)]
pub struct WallpaperManager {
    last_wallpaper: Option<PathBuf>,
    sequential_index: usize,
    wallpaper_cache: Vec<PathBuf>,
}

impl WallpaperManager {
    pub fn new() -> Self {
        Self {
            last_wallpaper: None,
            sequential_index: 0,
            wallpaper_cache: Vec::new(),
        }
    }

    pub fn get_wallpaper(&mut self, profile: &Profile, config: &Config) -> Result<String> {
        if self.wallpaper_cache.is_empty() {
            self.wallpaper_cache = self.collect_wallpapers(profile)?;
        }

        let wallpapers = &mut self.wallpaper_cache;
        
        if wallpapers.is_empty() {
            anyhow::bail!("No wallpapers found in configured directories");
        }

        let wallpaper = match config.auto_switch.mode {
            SwitchMode::Random => {
                let mut rng = rand::thread_rng();
                wallpapers.choose_mut(&mut rng).unwrap().clone()
            }
            SwitchMode::Sequential => {
                let wp = wallpapers[self.sequential_index % wallpapers.len()].clone();
                self.sequential_index += 1;
                wp
            }
        };

        Ok(wallpaper.to_string_lossy().to_string())
    }

    pub async fn set_wallpaper(&mut self, path: &str, profile: &Profile) -> Result<()> {
        info!("Setting wallpaper: {}", path);

        let output = Command::new("swww")
            .args(&[
                "img",
                path,
                "--transition-type",
                &profile.transition,
                "--transition-duration",
                &profile.transition_duration.to_string(),
            ])
            .output()
            .context("Failed to execute swww. Is swww daemon running? (swww init)")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("swww command failed: {}", stderr);
        }

        self.last_wallpaper = Some(PathBuf::from(path));
        Ok(())
    }

    pub fn last_wallpaper(&self) -> Option<&PathBuf> {
        self.last_wallpaper.as_ref()
    }

    pub fn refresh_cache(&mut self, profile: &Profile) -> Result<()> {
        self.wallpaper_cache = self.collect_wallpapers(profile)?;
        Ok(())
    }

    fn collect_wallpapers(&self, profile: &Profile) -> Result<Vec<PathBuf>> {
        let mut wallpapers = Vec::new();
        let extensions = ["jpg", "jpeg", "png", "gif", "webp", "bmp"];

        for dir in &profile.wallpaper_dirs {
            let dir = shellexpand::tilde(&dir.to_string_lossy()).into_owned();
            let dir = PathBuf::from(dir);
            
            if !dir.exists() {
                tracing::warn!("Wallpaper directory does not exist: {:?}", dir);
                continue;
            }

            for ext in &extensions {
                let pattern = format!("{}/*.{}", dir.display(), ext);
                if let Ok(paths) = glob(&pattern) {
                    for path in paths.flatten() {
                        wallpapers.push(path);
                    }
                }
                
                let pattern_upper = format!("{}/*.{}", dir.display(), ext.to_uppercase());
                if let Ok(paths) = glob(&pattern_upper) {
                    for path in paths.flatten() {
                        wallpapers.push(path);
                    }
                }
            }
        }

        wallpapers.sort();
        wallpapers.dedup();

        info!("Found {} wallpapers", wallpapers.len());
        Ok(wallpapers)
    }
}
