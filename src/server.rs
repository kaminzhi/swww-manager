use crate::config::Config;
use crate::monitor::MonitorManager;
use crate::wallpaper::WallpaperManager;
use crate::profile::ProfileManager;
use crate::protocol::{Request, Response, StatusInfo, ProfileInfo};
use crate::notify;

use anyhow::{Context, Result};
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::path::PathBuf;
use std::time::Instant;
use tracing::{info, error, warn, debug};

#[derive(Clone)]
pub struct Server {
    config: Config,
    monitor_manager: MonitorManager,
    wallpaper_manager: WallpaperManager,
    profile_manager: ProfileManager,
    start_time: Instant,
}

impl Server {
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing server with profile: {}", config.current_profile);
        
        Ok(Self {
            monitor_manager: MonitorManager::new(),
            wallpaper_manager: WallpaperManager::new(),
            profile_manager: ProfileManager::new(config.clone()),
            config,
            start_time: Instant::now(),
        })
    }

    pub async fn run(mut self) -> Result<()> {
        let socket_path = Self::socket_path();
        
        // Remove old socket if exists
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)
                .with_context(|| format!("Failed to remove old socket: {:?}", socket_path))?;
        }

        // Create socket directory
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create socket directory: {:?}", parent))?;
        }

        let listener = UnixListener::bind(&socket_path)
            .with_context(|| format!("Failed to bind socket at {:?}", socket_path))?;
        
        info!("Socket server listening at {:?}", socket_path);
        info!("Server ready to accept connections");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&socket_path, perms)?;
        }

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            debug!("Client connected: {:?}", addr);
                            let mut server = self.clone();
                            
                            tokio::spawn(async move {
                                if let Err(e) = server.handle_client(stream).await {
                                    error!("Client handler error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Accept error: {}", e);
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Received shutdown signal");
                    break;
                }
            }
        }

        info!("Shutting down server...");
        let _ = std::fs::remove_file(&socket_path);
        
        Ok(())
    }

    async fn handle_client(&mut self, mut stream: UnixStream) -> Result<()> {
        let mut buffer = vec![0u8; 8192];
        
        let n = match stream.read(&mut buffer).await {
            Ok(0) => {
                debug!("Client disconnected (EOF)");
                return Ok(());
            }
            Ok(n) => n,
            Err(e) => {
                error!("Read error: {}", e);
                return Err(e.into());
            }
        };

        let request: Request = serde_json::from_slice(&buffer[..n])
            .context("Failed to parse request JSON")?;
        
        info!("Processing request: {:?}", request);
       
        let response = self.process_request(request).await;
        
        debug!("Sending response: {:?}", response);
        
        let response_bytes = serde_json::to_vec(&response)
            .context("Failed to serialize response")?;
        
        stream.write_all(&response_bytes).await
            .context("Failed to write response")?;
        
        stream.flush().await
            .context("Failed to flush stream")?;
        
        Ok(())
    }

    async fn process_request(&mut self, request: Request) -> Response {
        match request {
            Request::Switch { profile } => {
                // Switch profile first if specified
                if let Some(prof) = profile {
                    if let Err(e) = self.switch_profile(&prof).await {
                        return Response::Error { 
                            message: format!("Failed to switch profile: {}", e)
                        };
                    }
                }
                
                // Then switch wallpaper
                match self.switch_wallpaper().await {
                    Ok(path) => {
                        let filename = std::path::Path::new(&path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(&path);
                        
                        notify::send_success(&format!("Wallpaper: {}", filename)).await.ok();
                        
                        Response::Success { 
                            message: format!("Switched to wallpaper: {}", filename) 
                        }
                    }
                    Err(e) => {
                        error!("Failed to switch wallpaper: {}", e);
                        notify::send_error(&e.to_string()).await.ok();
                        Response::Error { 
                            message: format!("Failed to switch wallpaper: {}", e)
                        }
                    }
                }
            }
            
            Request::SwitchProfile { name } => {
                match self.switch_profile(&name).await {
                    Ok(_) => {
                        Response::Success { 
                            message: format!("Switched to profile: {}", name) 
                        }
                    }
                    Err(e) => {
                        error!("Failed to switch profile: {}", e);
                        Response::Error { 
                            message: format!("Failed to switch profile: {}", e)
                        }
                    }
                }
            }
            
            Request::DetectAndSwitchProfile => {
                let monitors = match self.monitor_manager.get_monitors().await {
                    Ok(m) => m,
                    Err(e) => {
                        error!("Failed to get monitors: {}", e);
                        return Response::Error { 
                            message: format!("Failed to get monitors: {}", e)
                        };
                    }
                };
                
                info!("Detecting profile for monitors: {:?}", monitors);
                
                match self.profile_manager.detect_profile(&monitors) {
                    Ok(Some(profile)) => {
                        if profile != self.config.current_profile {
                            info!("Detected profile: {} (current: {})", profile, self.config.current_profile);
                            
                            if let Err(e) = self.switch_profile(&profile).await {
                                return Response::Error { 
                                    message: format!("Failed to switch to detected profile: {}", e)
                                };
                            }
                            
                            Response::Success { 
                                message: format!("Auto-switched to profile: {}", profile) 
                            }
                        } else {
                            Response::Success { 
                                message: format!("Already using optimal profile: {}", profile)
                            }
                        }
                    }
                    Ok(None) => {
                        warn!("No matching profile found for monitors: {:?}", monitors);
                        Response::Success { 
                            message: "No matching profile found, using current".to_string()
                        }
                    }
                    Err(e) => {
                        error!("Failed to detect profile: {}", e);
                        Response::Error { 
                            message: format!("Failed to detect profile: {}", e)
                        }
                    }
                }
            }
            
            Request::ListProfiles => {
                let profiles = self.profile_manager.get_profile_list();
                Response::ProfileList { profiles }
            }
            
            Request::GetStatus => {
                let monitors = self.monitor_manager.get_monitors().await.unwrap_or_default();
                
                let status = StatusInfo {
                    current_profile: self.config.current_profile.clone(),
                    current_wallpaper: self.wallpaper_manager.last_wallpaper()
                        .map(|p| p.to_string_lossy().to_string()),
                    auto_switch_enabled: self.config.auto_switch.enabled,
                    auto_switch_interval: Some(self.config.auto_switch.interval),
                    monitors,
                    uptime_secs: self.start_time.elapsed().as_secs(),
                };
                
                Response::Status { status }
            }
            
            Request::SetAutoSwitch { enabled } => {
                self.config.auto_switch.enabled = enabled;
                
                if let Err(e) = self.config.save(None) {
                    error!("Failed to save config: {}", e);
                    return Response::Error { 
                        message: format!("Failed to save config: {}", e)
                    };
                }
                
                let status = if enabled { "enabled" } else { "disabled" };
                info!("Auto-switch {}", status);
                
                Response::Success { 
                    message: format!("Auto-switch {}", status)
                }
            }
            
            Request::SetAutoSwitchInterval { interval } => {
                self.config.auto_switch.interval = interval;
                
                if let Err(e) = self.config.save(None) {
                    error!("Failed to save config: {}", e);
                    return Response::Error { 
                        message: format!("Failed to save config: {}", e)
                    };
                }
                
                info!("Auto-switch interval set to {}s", interval);
                
                Response::Success { 
                    message: format!("Auto-switch interval set to {}s ({} minutes)", 
                        interval, interval / 60)
                }
            }
            
            Request::ReloadConfig => {
                match Config::load(None) {
                    Ok(new_config) => {
                        info!("Reloading configuration");
                        self.config = new_config.clone();
                        self.profile_manager.update_config(new_config);
                        
                        // Refresh wallpaper cache
                        if let Ok(profile) = self.profile_manager.current_profile() {
                            if let Err(e) = self.wallpaper_manager.refresh_cache(profile) {
                                warn!("Failed to refresh wallpaper cache: {}", e);
                            }
                        }
                        
                        Response::Success { 
                            message: "Configuration reloaded".to_string()
                        }
                    }
                    Err(e) => {
                        error!("Failed to reload config: {}", e);
                        Response::Error { 
                            message: format!("Failed to reload config: {}", e)
                        }
                    }
                }
            }
            
            Request::Shutdown => {
                info!("Shutdown requested");
                
                Response::Success { 
                    message: "Server shutting down".to_string()
                }
            }
        }
    }

    async fn switch_wallpaper(&mut self) -> Result<String> {
        let profile = self.profile_manager.current_profile()
            .context("Failed to get current profile")?;
        
        // Refresh wallpaper cache to pick up new images
        self.wallpaper_manager.refresh_cache(profile)
            .context("Failed to refresh wallpaper cache")?;
        
        let wallpaper = self.wallpaper_manager.get_wallpaper(profile, &self.config)
            .context("Failed to get wallpaper")?;
        
        info!("Switching to wallpaper: {}", wallpaper);
        
        self.wallpaper_manager.set_wallpaper(&wallpaper, profile).await
            .context("Failed to set wallpaper")?;
        
        Ok(wallpaper)
    }

    async fn switch_profile(&mut self, name: &str) -> Result<()> {
        info!("Switching to profile: {}", name);
        
        self.profile_manager.switch_to(name)
            .with_context(|| format!("Profile '{}' not found", name))?;
        
        self.config.current_profile = name.to_string();
        self.config.save(None)
            .context("Failed to save config after profile switch")?;
        
        notify::send("Profile switched", name).await
            .context("Failed to send notification")?;
        
        // Switch wallpaper immediately
        self.switch_wallpaper().await?;
        
        Ok(())
    }

    fn socket_path() -> PathBuf {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("/run/user/{}", users::get_current_uid()));
        
        PathBuf::from(runtime_dir).join("swww-manager.sock")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_server_creation() {
        let config = Config::default();
        let server = Server::new(config).await;
        assert!(server.is_ok());
    }

    #[tokio::test]
    async fn test_socket_path() {
        let path = Server::socket_path();
        assert!(path.ends_with("swww-manager.sock"));
    }
}
