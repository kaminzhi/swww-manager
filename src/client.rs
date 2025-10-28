use crate::protocol::{Request, Response};
use anyhow::{Context, Result};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::path::PathBuf;
use tracing::info;

pub struct Client {
    stream: UnixStream,
}

impl Client {
    pub async fn connect() -> Result<Self> {
        let socket_path = Self::socket_path();
        
        let stream = UnixStream::connect(&socket_path)
            .await
            .context("Failed to connect to socket. Is the service running?\n\
                     Try: systemctl --user start swww-manager.socket")?;
        
        Ok(Self { stream })
    }

    async fn send_request(&mut self, request: Request) -> Result<Response> {
        let request_bytes = serde_json::to_vec(&request)?;
        self.stream.write_all(&request_bytes).await?;
        self.stream.flush().await?;
        
        let mut buffer = vec![0u8; 8192];
        let n = self.stream.read(&mut buffer).await?;
        
        if n == 0 {
            anyhow::bail!("Server closed connection");
        }
        
        let response: Response = serde_json::from_slice(&buffer[..n])?;
        Ok(response)
    }

    pub async fn switch_wallpaper(&mut self, profile: Option<&str>) -> Result<()> {
        let request = Request::Switch { 
            profile: profile.map(String::from) 
        };
        
        match self.send_request(request).await? {
            Response::Success { message } => {
                println!("{}", message);
                Ok(())
            }
            Response::Error { message } => {
                anyhow::bail!("Error: {}", message)
            }
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub async fn switch_profile(&mut self, name: &str) -> Result<()> {
        let request = Request::SwitchProfile { 
            name: name.to_string() 
        };
        
        match self.send_request(request).await? {
            Response::Success { message } => {
                println!("{}", message);
                Ok(())
            }
            Response::Error { message } => {
                anyhow::bail!("Error: {}", message)
            }
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub async fn list_profiles(&mut self, detailed: bool) -> Result<()> {
        let request = Request::ListProfiles;
        
        match self.send_request(request).await? {
            Response::ProfileList { profiles } => {
                println!("\nAvailable Profiles:");
                println!("{}", "─".repeat(70));
                
                for profile in profiles {
                    let marker = if profile.is_current { "Founded" } else { " " };
                    println!("\n[{}] {}", marker, profile.name);
                    
                    if detailed {
                        println!("Monitors: {}", profile.monitors.join(", "));
                        println!("Wallpapers: {}", profile.wallpaper_count);
                        if let Some(transition) = &profile.transition {
                            println!("Transition: {} ({}s)", 
                                transition, profile.transition_duration.unwrap_or(2));
                        }
                    } else {
                        print!("{} monitor(s)", profile.monitors.len());
                        println!("{} wallpaper(s)", profile.wallpaper_count);
                    }
                }
                println!();
                Ok(())
            }
            Response::Error { message } => {
                anyhow::bail!("Error: {}", message)
            }
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub async fn get_status(&mut self, json: bool) -> Result<()> {
        let request = Request::GetStatus;
        
        match self.send_request(request).await? {
            Response::Status { status } => {
                if json {
                    println!("{}", serde_json::to_string_pretty(&status)?);
                } else {
                    println!("\nStatus:");
                    println!("{}", "─".repeat(70));
                    println!("Profile:      {}", status.current_profile);
                    println!("Wallpaper:    {}", status.current_wallpaper
                        .as_ref()
                        .and_then(|p| std::path::Path::new(p).file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("None"));
                    println!("Auto-switch:  {}", 
                        if status.auto_switch_enabled { "Enabled" } else { "Disabled" });
                    println!("Monitors:     {}", status.monitors.join(", "));
                    println!("Uptime:       {}s", status.uptime_secs);
                    println!();
                }
                Ok(())
            }
            Response::Error { message } => {
                anyhow::bail!("Error: {}", message)
            }
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub async fn set_auto_switch(&mut self, enabled: bool) -> Result<()> {
        let request = Request::SetAutoSwitch { enabled };
        
        match self.send_request(request).await? {
            Response::Success { message } => {
                println!("{}", message);
                Ok(())
            }
            Response::Error { message } => {
                anyhow::bail!("Error: {}", message)
            }
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub async fn set_auto_switch_interval(&mut self, interval: u64) -> Result<()> {
        let request = Request::SetAutoSwitchInterval { interval };
        
        match self.send_request(request).await? {
            Response::Success { message } => {
                println!("{}", message);
                Ok(())
            }
            Response::Error { message } => {
                anyhow::bail!("Error: {}", message)
            }
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub async fn get_auto_switch_status(&mut self) -> Result<()> {
        let request = Request::GetStatus;
        
        match self.send_request(request).await? {
            Response::Status { status } => {
                println!("\nAuto-switch Status:");
                println!("{}", "─".repeat(70));
                println!("Enabled:  {}", 
                    if status.auto_switch_enabled { "Yes" } else { "No" });
                if let Some(interval) = status.auto_switch_interval {
                    println!("Interval: {}s ({} minutes)", interval, interval / 60);
                }
                println!();
                Ok(())
            }
            Response::Error { message } => {
                anyhow::bail!("Error: {}", message)
            }
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub async fn detect_and_switch_profile(&mut self) -> Result<()> {
        let request = Request::DetectAndSwitchProfile;
        
        match self.send_request(request).await? {
            Response::Success { message } => {
                info!(message);
                println!("{}", message);
                Ok(())
            }
            Response::Error { message } => {
                anyhow::bail!("Error: {}", message)
            }
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub async fn reload_config(&mut self) -> Result<()> {
        let request = Request::ReloadConfig;
        
        match self.send_request(request).await? {
            Response::Success { message } => {
                println!("{}", message);
                Ok(())
            }
            Response::Error { message } => {
                anyhow::bail!("Error: {}", message)
            }
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    fn socket_path() -> PathBuf {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("/run/user/{}", users::get_current_uid()));
        
        PathBuf::from(runtime_dir).join("swww-manager.sock")
    }
}
