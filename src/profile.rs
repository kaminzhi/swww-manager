use crate::config::{Config, Profile};
use crate::protocol::ProfileInfo;
use anyhow::{Context, Result};
use std::collections::HashSet;

#[derive(Clone)]
pub struct ProfileManager {
    config: Config,
}

impl ProfileManager {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn current_profile(&self) -> Result<&Profile> {
        self.config
            .profiles
            .get(&self.config.current_profile)
            .context("Current profile not found")
    }

    pub fn switch_to(&mut self, name: &str) -> Result<()> {
        if !self.config.profiles.contains_key(name) {
            anyhow::bail!("Profile '{}' not found", name);
        }

        self.config.current_profile = name.to_string();
        Ok(())
    }

    pub fn detect_profile(&self, monitors: &[String]) -> Result<Option<String>> {
        let monitor_set: HashSet<_> = monitors.iter().collect();

        let mut best_match = None;
        let mut best_score = 0;

        for (name, profile) in &self.config.profiles {

            if profile.monitors.len() == 1 && profile.monitors.contains(&"*".to_string()) {
                if best_match.is_none() {
                    best_match = Some(name.clone());
                }
                continue;
            }

            let profile_monitors: HashSet<_> = profile.monitors.iter().collect();

            if monitor_set.len() != profile_monitors.len() {
                continue;
            }
            if monitor_set == profile_monitors {
                let score = monitor_set.len();
                
                if score > best_score {
                    best_score = score;
                    best_match = Some(name.clone());
                }
            }
        }

        Ok(best_match)
    }

    pub fn list(&self) {
        println!("\nAvailable Profiles:");
        println!("{}", "-".repeat(50));

        for (name, profile) in &self.config.profiles {
            let current = if name == &self.config.current_profile { "✓" } else { " " };
            println!("[{}] {}", current, name);
            println!("Monitors: {}", profile.monitors.join(", "));
            println!("Wallpaper dirs: {}", profile.wallpaper_dirs.len());
            println!("Transition: {} ({}s)", profile.transition, profile.transition_duration);
            println!();
        }
    }

    pub fn get_profile_list(&self) -> Vec<ProfileInfo> {
        self.config
            .profiles
            .iter()
            .map(|(name, profile)| {
                // Count wallpapers
                let wallpaper_count = profile
                    .wallpaper_dirs
                    .iter()
                    .filter_map(|dir| {
                        let extensions = ["jpg", "jpeg", "png", "gif", "webp"];
                        let mut count = 0;
                        for ext in &extensions {
                            let pattern = format!("{}/*.{}", dir.display(), ext);
                            if let Ok(paths) = glob::glob(&pattern) {
                                count += paths.count();
                            }
                        }
                        Some(count)
                    })
                    .sum();

                ProfileInfo {
                    name: name.clone(),
                    monitors: profile.monitors.clone(),
                    wallpaper_count,
                    is_current: name == &self.config.current_profile,
                    transition: Some(profile.transition.clone()),
                    transition_duration: Some(profile.transition_duration),
                }
            })
            .collect()
    }

    pub fn update_config(&mut self, config: Config) {
        self.config = config;
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
