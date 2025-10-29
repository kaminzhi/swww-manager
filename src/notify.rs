use crate::hyprland_ipc::HyprlandIPC;
use anyhow::Result;
use tracing::warn;

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum NotificationKind {
    Info,
    Success,
    Warning,
    Error,
    Hint,
}

fn icon_for(kind: NotificationKind) -> i32 {
    // Hyprland notify icon levels: -1 = none, 0 = warning, 1 = info, 2 = hint, 3 = error, 4 = confused
    match kind {
        NotificationKind::Warning => 0,
        NotificationKind::Info => 1,
        NotificationKind::Hint => 2,
        NotificationKind::Error => 3,
        NotificationKind::Success => 1,
    }
}

pub async fn send(title: &str, message: &str) -> Result<()> {
    let text = format!("{}: {}", title, message);
    send_with_color(NotificationKind::Info, &text, "rgb(88ccff)", 5000).await
}

pub async fn send_error(message: &str) -> Result<()> {
    let text = message.to_string();
    send_with_color(NotificationKind::Error, &text, "rgb(ff8888)", 8000).await
}

pub async fn send_success(message: &str) -> Result<()> {
    let text = message.to_string();
    send_with_color(NotificationKind::Success, &text, "rgb(88ff88)", 3000).await
}

async fn send_with_color(kind: NotificationKind, message: &str, color: &str, duration_ms: u32) -> Result<()> {
    match HyprlandIPC::new() {
        Ok(ipc) => {
            let icon = icon_for(kind);
            ipc.notify(icon, duration_ms, color, message).await?;
        }
        Err(e) => {
            warn!("Failed to send notification: {}", e);
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub fn send_sync(title: &str, message: &str) -> Result<()> {
    let title = title.to_owned();
    let message = message.to_owned();

    if let Ok(handle) = tokio::runtime::Handle::try_current() { handle.spawn(async move {
                send(&title, &message).await.ok();
            }); }
    Ok(())
}
