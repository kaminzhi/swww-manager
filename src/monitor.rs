use crate::hyprland_ipc::{HyprlandIPC, Monitor as HyprMonitor};
use anyhow::Result;
use tracing::warn;

#[derive(Clone)]
pub struct MonitorManager {
    ipc: Option<HyprlandIPC>,
}

impl MonitorManager {
    pub fn new() -> Self {
        let ipc = match HyprlandIPC::new() {
            Ok(ipc) => Some(ipc),
            Err(e) => {
                warn!("Failed to initialize Hyprland IPC: {}. Monitor detection disabled.", e);
                None
            }
        };

        Self { ipc }
    }

    pub async fn get_monitors(&self) -> Result<Vec<String>> {
        if let Some(ipc) = &self.ipc {
            let monitors = ipc.get_monitors().await?;
            Ok(monitors.into_iter().map(|m| m.name).collect())
        } else {
            anyhow::bail!("Hyprland IPC not available")
        }
    }

    pub async fn get_monitor_details(&self) -> Result<Vec<HyprMonitor>> {
        if let Some(ipc) = &self.ipc {
            ipc.get_monitors().await
        } else {
            anyhow::bail!("Hyprland IPC not available")
        }
    }

    pub async fn get_focused_monitor(&self) -> Result<String> {
        if let Some(ipc) = &self.ipc {
            let monitors = ipc.get_monitors().await?;
            monitors
                .into_iter()
                .find(|m| m.focused)
                .map(|m| m.name)
                .ok_or_else(|| anyhow::anyhow!("No focused monitor found"))
        } else {
            anyhow::bail!("Hyprland IPC not available")
        }
    }
}
