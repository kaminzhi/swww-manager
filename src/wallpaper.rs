use crate::config::{Config, Profile, SwitchMode};
use anyhow::{Context, Result};
use glob::glob;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::info;
use tokio::time::{timeout, Duration};

#[derive(Clone)]
pub struct WallpaperManager {
    last_wallpaper: Option<PathBuf>,
    sequential_index: usize,
    wallpaper_cache: Vec<PathBuf>,
}

impl Default for WallpaperManager {
    fn default() -> Self {
        Self::new()
    }
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

        // if only one wallpaper, just return it
        if wallpapers.len() == 1 {
            return Ok(wallpapers[0].to_string_lossy().to_string());
        }

        let chosen_path = match config.auto_switch.mode {
            SwitchMode::Random => {
                // use rand::random::<u32>() % len to avoid thread_rng/gen_range deprecation warnings
                let mut attempts = 0;
                loop {
                    let idx = (rand::random::<u32>() as usize) % wallpapers.len();
                    let cand = wallpapers[idx].clone();
                    if self.last_wallpaper.as_ref().map(|p| p != &cand).unwrap_or(true) {
                        break cand;
                    }
                    attempts += 1;
                    if attempts >= 8 {
                        break cand;
                    }
                }
            }
            SwitchMode::Sequential => {
                // advance at least one slot; choose first index not equal to last_wallpaper
                let mut start = self.sequential_index % wallpapers.len();
                let mut found = None;
                for _ in 0..wallpapers.len() {
                    let cand = wallpapers[start].clone();
                    if self.last_wallpaper.as_ref().map(|p| p != &cand).unwrap_or(true) {
                        found = Some(cand);
                        // next time start from next position
                        self.sequential_index = (start + 1) % wallpapers.len();
                        break;
                    }
                    start = (start + 1) % wallpapers.len();
                }
                // fallback to current index if nothing found (shouldn't happen)
                found.unwrap_or_else(|| {
                    let idx = self.sequential_index % wallpapers.len();
                    let wp = wallpapers[idx].clone();
                    self.sequential_index = (self.sequential_index + 1) % wallpapers.len();
                    wp
                })
            }
        };

        Ok(chosen_path.to_string_lossy().to_string())
    }

    pub async fn set_wallpaper(&mut self, path: &str, profile: &Profile) -> Result<()> {
        info!("Setting wallpaper: {}", path);

        let cmd = Command::new("swww")
            .args([
                "img",
                path,
                "--transition-type",
                &profile.transition,
                "--transition-duration",
                &profile.transition_duration.to_string(),
            ])
            .output();

        let output = match timeout(Duration::from_secs(6), cmd).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(e).context("Failed to execute swww. Is swww daemon running? (swww init)")?;
            }
            Err(_) => {
                anyhow::bail!("swww command timed out");
            }
        };

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

    pub fn set_last_wallpaper(&mut self, path: PathBuf) {
        self.last_wallpaper = Some(path);
    }
    
    pub fn refresh_cache(&mut self, profile: &Profile) -> Result<()> {
        self.wallpaper_cache = self.collect_wallpapers(profile)?;
        Ok(())
    }

    pub async fn ensure_cache(&mut self, profile: &Profile) -> Result<()> {
        if !self.wallpaper_cache.is_empty() {
            return Ok(());
        }

        let dirs: Vec<PathBuf> = profile
            .wallpaper_dirs
            .iter()
            .map(|d| {
                let dir = shellexpand::tilde(&d.to_string_lossy()).into_owned();
                PathBuf::from(dir)
            })
            .collect();

        let wallpapers = tokio::task::spawn_blocking(move || -> Result<Vec<PathBuf>> {
            let mut wallpapers = Vec::new();
            let extensions = ["jpg", "jpeg", "png", "gif", "webp", "bmp"];

            for dir in dirs {
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
            Ok(wallpapers)
        })
        .await
        .map_err(|e| anyhow::anyhow!("Join error when collecting wallpapers: {}", e))??;

        self.wallpaper_cache = wallpapers;
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
