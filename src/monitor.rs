use crate::hyprland_ipc::{HyprlandIPC, Monitor as HyprMonitor};
use anyhow::{anyhow, Result};
use tracing::warn;

#[derive(Clone)]
pub struct MonitorManager {
    ipc: Option<HyprlandIPC>,
}

impl Default for MonitorManager {
    fn default() -> Self {
        Self::new()
    }
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
            Ok(monitors
                .into_iter()
                .filter(|m| m.dpmsStatus && m.width > 0 && m.height > 0)
                .map(|m| m.activeWorkspace.name.clone())
                .collect())
        } else {
            Err(anyhow!("Hyprland IPC not available"))
        }
    }

    pub async fn get_monitor_details(&self) -> Result<Vec<HyprMonitor>> {
        if let Some(ipc) = &self.ipc {
            let monitors = ipc.get_monitors().await?;
            Ok(monitors
                .into_iter()
                .filter(|m| m.dpmsStatus && m.width > 0 && m.height > 0)
                .collect())
        } else {
            Err(anyhow!("Hyprland IPC not available"))
        }
    }

    pub async fn get_focused_monitor(&self) -> Result<String> {
        if let Some(ipc) = &self.ipc {
            let monitors = ipc.get_monitors().await?;
            monitors
                .into_iter()
                .find(|m| m.focused)
                .map(|m| m.activeWorkspace.name.clone())
                .ok_or_else(|| anyhow!("No focused monitor found"))
        } else {
            Err(anyhow!("Hyprland IPC not available"))
        }
    }
    
    pub async fn get_stable_monitors(&self) -> Result<Vec<String>> {
        use tokio::time::{sleep, Duration, Instant};
        let total = Duration::from_millis(1200);
        let step = Duration::from_millis(200);
        let required_same = 3usize;

        let start = Instant::now();
        let mut last: Option<Vec<String>> = None;
        let mut same = 0usize;

        while start.elapsed() < total {
            let mut current = self.get_monitors().await.unwrap_or_default();
            current.sort();
            if let Some(prev) = &last {
                if prev == &current {
                    same += 1;
                    if same >= required_same {
                        return Ok(current);
                    }
                } else {
                    same = 1;
                    last = Some(current);
                }
            } else {
                same = 1;
                last = Some(current);
            }
            sleep(step).await;
        }

        Ok(last.unwrap_or_default())
    }
}
