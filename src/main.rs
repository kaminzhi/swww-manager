mod hyprland_events;

fn main() {
    println!("Hello, world!");
}

async fn run_event_monitor() -> Result<()> {
    use hyprland_events::{monitor_events, HyprlandEvent};
    use futures::FutureExt;
    
    monitor_events(|event| {
        async move {
            match event {
                HyprlandEvent::MonitorAdded { name, .. } => {
                    info!("Monitor added: {}", name);
                    notify::send("Monitor added", &name).ok();
                    
                    // Trigger profile detection
                    if let Ok(mut client) = Client::connect().await {
                        client.detect_and_switch_profile().await.ok();
                    }
                }
                HyprlandEvent::MonitorRemoved { name, .. } => {
                    info!("Monitor removed: {}", name);
                    notify::send("Monitor removed", &name).ok();
                    
                    // Trigger profile detection
                    if let Ok(mut client) = Client::connect().await {
                        client.detect_and_switch_profile().await.ok();
                    }
                }
                _ => {}
            }
        }.boxed()
    }).await
}
