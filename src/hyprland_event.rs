use anyhow::{Context, Result};
use tokio::net::UnixStream;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{info, warn, error};
use std::path::Pathbuf;

#[derive(Debug, clone, PartialEq)]
pub enum HyprlandEvent {
    MonitorAdded { id: String, name: String, description: String },
    MonitorRemoved { id: String, name: String, description: String },
    Workspace { id: String, name: String },
    FocusedMon { monitor: String, workspace: String  },
    Other(String),
}

pub struct HyprlandEvent {
    reader: BufReader<UnixStream>,
}

impl HyprlandEvent {
    pub async fn connect() -> Result<self> {
        let socket_patrh = Self::socket2_path()?;
        let stream = UnixStream::connect(&socket_path)
            .await
            .context("Failed to connect socket")?;

        info!("Connected socket at {:?}", socket_path);
        
        Ok(Self {
            reader: BufReader::new(stream),
        })

    }

    pub async fn next_event (&mut self) -> Result<HyprlandEvent> {
        let mut line = String::new();

        match self.reader.read_line(&mut line).await {
            Ok(0) => OK(None),
            Ok(_) => {
                let event = Self::parse_event(&line);
                Ok(Some(event))
            },
            Err(e) => Err(anyhow::anyhow!("Failed to read from socket: {}", e.into())),
        }
    }

    fn parse_eventfn parse_event(line: &str) -> Result<HyprlandEvent> {
        let line = line.trim();
        
        if let Some((event_type, data)) = line.split_once(">>") {
            let event = match event_type {
                "monitoraddedv2" => {
                    let parts: Vec<&str> = data.split(',').collect();
                    if parts.len() >= 3 {
                        HyprlandEvent::MonitorAdded {
                            id: parts[0].to_string(),
                            name: parts[1].to_string(),
                            description: parts[2..].join(","),
                        }
                    } else {
                        HyprlandEvent::Other(line.to_string())
                    }
                }
                "monitorremovedv2" => {
                    let parts: Vec<&str> = data.split(',').collect();
                    if parts.len() >= 3 {
                        HyprlandEvent::MonitorRemoved {
                            id: parts[0].to_string(),
                            name: parts[1].to_string(),
                            description: parts[2..].join(","),
                        }
                    } else {
                        HyprlandEvent::Other(line.to_string())
                    }
                }
                "workspacev2" => {
                    let parts: Vec<&str> = data.split(',').collect();
                    if parts.len() >= 2 {
                        HyprlandEvent::Workspace {
                            id: parts[0].to_string(),
                            name: parts[1].to_string(),
                        }
                    } else {
                        HyprlandEvent::Other(line.to_string())
                    }
                }
                "focusedmon" => {
                    let parts: Vec<&str> = data.split(',').collect();
                    if parts.len() >= 2 {
                        HyprlandEvent::FocusedMon {
                            monitor: parts[0].to_string(),
                            workspace: parts[1].to_string(),
                        }
                    } else {
                        HyprlandEvent::Other(line.to_string())
                    }
                }
                _ => HyprlandEvent::Other(line.to_string()),
            };
            Ok(event)
        } else {
            Ok(HyprlandEvent::Other(line.to_string()))
        }
    }

    fn socket2_path() -> Result<PathBuf> {
        let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .context("HYPRLAND_INSTANCE_SIGNATURE not set")?;
        
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| "/run/user/1000".to_string());
        
        Ok(PathBuf::from(runtime_dir)
            .join("hypr")
            .join(his)
            .join(".socket2.sock"))
    }
}

pub async fn monitor_events<F>(mut handler: F) -> Result<()>
where
    F: FnMut(HyprlandEvent) -> futures::future::BoxFuture<'static, ()>,
{
    let mut listener = EventListener::connect().await?;
    
    info!("Starting event monitoring...");
    
    loop {
        match listener.next_event().await {
            Ok(Some(event)) => {
                handler(event).await;
            }
            Ok(None) => {
                warn!("Event stream ended, reconnecting...");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                listener = EventListener::connect().await?;
            }
            Err(e) => {
                error!("Event error: {}, reconnecting...", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                listener = EventListener::connect().await?;
            }
        }
    }
}
