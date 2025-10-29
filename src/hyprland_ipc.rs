use anyhow::{Context, Result};
use serde::Deserialize;
use tokio::net::UnixStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::path::PathBuf;

#[derive(Clone)]
pub struct HyprlandIPC {
    socket_path: PathBuf,
}

impl HyprlandIPC {
    pub fn new() -> Result<Self> {
        let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .context("HYPRLAND_INSTANCE_SIGNATURE not set. Are you running under Hyprland?")?;
        
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("/run/user/{}", users::get_current_uid()));
        
        let socket_path = PathBuf::from(runtime_dir)
            .join("hypr")
            .join(his)
            .join(".socket.sock");

        if !socket_path.exists() {
            anyhow::bail!("Hyprland socket not found at {:?}", socket_path);
        }

        Ok(Self { socket_path })
    }

    pub async fn dispatch(&self, command: &str) -> Result<String> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .context("Failed to connect to Hyprland socket")?;

        stream.write_all(command.as_bytes()).await?;
        stream.flush().await?;

        let mut response = String::new();
        stream.read_to_string(&mut response).await?;

        Ok(response)
    }

    /// dispatch_json 
    pub async fn dispatch_json(&self, command: &str) -> Result<String> {
        let cmd = format!("j/{}", command);
        self.dispatch(&cmd).await
    }

    /// get_monitors
    pub async fn get_monitors(&self) -> Result<Vec<Monitor>> {
        let response = self.dispatch_json("monitors").await?;
        let monitors: Vec<Monitor> = serde_json::from_str(&response)
            .context("Failed to parse monitors JSON")?;
        Ok(monitors)
    }

    /// notify
    pub async fn notify(&self, icon: i32, duration_ms: u32, color: &str, message: &str) -> Result<()> {
        let cmd = format!("notify {} {} {} {}", icon, duration_ms, color, message);
        self.dispatch(&cmd).await?;
        Ok(())
    }
    
    /*
    /// activeWorkspace
    pub async fn get_active_workspace(&self) -> Result<Workspace> {
        let response = self.dispatch_json("activeworkspace").await?;
        let workspace: Workspace = serde_json::from_str(&response)
            .context("Failed to parse workspace JSON")?;
        Ok(workspace)
    }

    /// dispatch
    pub async fn exec_dispatch(&self, dispatcher: &str, args: &str) -> Result<()> {
        let cmd = format!("dispatch {} {}", dispatcher, args);
        self.dispatch(&cmd).await?;
        Ok(())
    }
    */
}

#[allow(non_snake_case, dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Monitor {
    pub id: i32,
    // pub name: String,
    // pub description: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub width: i32,
    pub height: i32,
    pub refreshRate: f32,
    pub x: i32,
    pub y: i32,
    pub activeWorkspace: WorkspaceBasic,
    pub reserved: Vec<i32>,
    pub scale: f32,
    pub transform: i32,
    pub focused: bool,
    pub dpmsStatus: bool,
    pub vrr: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceBasic {
    pub id: i32,
    pub name: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    pub id: i32,
    pub name: String,
    pub monitor: String,
    pub windows: i32,
    pub hasfullscreen: bool,
    pub lastwindow: String,
    pub lastwindowtitle: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Only run in Hyprland environment
    async fn test_get_monitors() {
        let ipc = HyprlandIPC::new().unwrap();
        let monitors = ipc.get_monitors().await.unwrap();
        assert!(!monitors.is_empty());
    }
}
