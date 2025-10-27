use crate::hyprland_ipc::HyprlandIPC;
use anyhow::Result;
use tracing::warn;``
// use std::process::Command;

pub async fn send(title: &str, message: &str) -> Result<()> {
    let text = format!("{}: {}", title, message);
    send_with_color(&text, "rgb(88ccff)", 5000).await
}

pub async fn send_error(message: &str) -> Result<()> {
    let text = format!("Error: {}", message);
    send_with_color(&text, "rgb(ff8888)", 8000).await
}

pub async fn send_success(message: &str) -> Result<()> {
    let text = format!("Success: {}", message);
    send_with_color(&text, "rgb(88ff88)", 3000).await
}

async fn send_with_color(message: &str, color: &str, duration_ms: u32) -> Result<()> {
    match HyprlandIPC::new() {
        Ok(ipc) => {
            // Icon levels: -1 = none, 0 = warning, 1 = info, 2 = hint, 3 = error, 4 = confused
            let icon = if message.contains("Error") {
                3
            } else if message.contains("Success") {
                1
            } else {
                2
            };

            ipc.notify(icon, duration_ms, color, message).await?;
        }
        Err(e) => {
            warn!("Failed to send notification: {}", e);
        }
    }
    Ok(())
}

pub fn send_sync(title: &str, message: &str) -> Result<()> {
    tokio::runtime::Handle::try_current()
        .ok()
        .map(|handle| {
            handle.spawn(async move {
                send(title, message).await.ok();
            });
        });
    Ok(())
}
