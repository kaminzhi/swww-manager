use anyhow::{Context, Result};
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
struct HyprMonitor {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone)]
pub struct MonitorManager;

impl MonitorManager {
    pub fn new() -> Self {
        Self
    }

    pub fn get_monitors(&self) -> Result<Vec<String>> {
        let output = Command::new("hyprctl")
            .args(&["monitors", "-j"])
            .output()
            .context("Failed to execute hyprctl")?;

        if !output.status.success() {
            anyhow::bail!("hyprctl command failed");
        }

        let monitors: Vec<HyprMonitor> = serde_json::from_slice(&output.stdout)
            .context("Failed to parse hyprctl output")?;

        Ok(monitors.into_iter().map(|m| m.name).collect())
    }

    pub fn get_monitor_details(&self) -> Result<Vec<HyprMonitor>> {
        let output = Command::new("hyprctl")
            .args(&["monitors", "-j"])
            .output()
            .context("Failed to execute hyprctl")?;

        if !output.status.success() {
            anyhow::bail!("hyprctl command failed");
        }

        let monitors: Vec<HyprMonitor> = serde_json::from_slice(&output.stdout)
            .context("Failed to parse hyprctl output")?;

        Ok(monitors)
    }
}
