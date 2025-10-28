use tracing_subscriber;
use tracing::Level;
use tracing::info;
use clap::Subcommand;

mod config;
mod monitor;
mod wallpaper;
mod profile;
mod server;
mod client;
mod protocol;
mod hyprland_event;
mod hyprland_ipc;
mod notify;

use clap::Parser;
use config::Config;
use client::Client;
use server::Server;
// use hyprland_event::{monitor_events, HyprlandEvent};
use futures::FutureExt;
use anyhow::Result;

#[derive(Parser)]
#[command(
    name = "swww-manager",
    author = "SWWW Manager Contributors",
    version = env!("CARGO_PKG_VERSION"),
    about = "Advanced wallpaper manager for Hyprland with swww",
)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,

    #[arg(short, long, value_name = "FILE", global = true)]
    config: Option<String>,

    #[arg(short, long, global = true)]
    debug: bool,

    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    Serve,
    
    #[command(name = "monitor-events")]
    MonitorEvents,
    
    Switch {
        #[arg(short, long)]
        profile: Option<String>,
        
        #[arg(short, long)]
        random: bool,
        
        #[arg(short = 'n', long)]
        next: bool,
    },
    
    List {
        #[arg(short, long)]
        detailed: bool,
    },
    
    /// Switch to a specific profile
    Profile {
        name: String,
    },
    
    /// Show current status
    Status {
        #[arg(short, long)]
        json: bool,
    },
    
    /// Control auto-switch feature
    Auto {
        /// Action: on, off, or status
        #[arg(value_parser = ["on", "off", "status"])]
        action: String,
        
        #[arg(short, long)]
        interval: Option<u64>,
    },
    
    /// Initialize configuration file
    Init {
        #[arg(short, long)]
        force: bool,
    },
    
    /// Reload configuration
    Reload,
    
    /// Detect and switch to optimal profile
    Detect,
    
    /// Show information about monitors
    Monitors {
        /// Watch for monitor changes
        #[arg(short, long)]
        watch: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.debug {
        Level::DEBUG
    } else if cli.verbose {
        Level::INFO
    } else {
        Level::WARN
    };
    
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .init();

    // Execute command
    match cli.command {
        Commands::Serve => {
            info!("Starting socket server...");
            let config = Config::load(cli.config.as_deref())?;
            let server = Server::new(config).await?;
            server.run().await?;
        }
        
        Commands::MonitorEvents => {
            info!("Starting Hyprland event monitor...");
            run_event_monitor().await?;
        }
        
        Commands::Switch { profile, random: _, next: _ } => {
            let mut client = Client::connect().await?;
            client.switch_wallpaper(profile.as_deref()).await?;
        }
        
        Commands::List { detailed } => {
            let mut client = Client::connect().await?;
            client.list_profiles(detailed).await?;
        }
        
        Commands::Profile { name } => {
            let mut client = Client::connect().await?;
            client.switch_profile(&name).await?;
        }
        
        Commands::Status { json } => {
            let mut client = Client::connect().await?;
            client.get_status(json).await?;
        }
        
        Commands::Auto { action, interval } => {
            let mut client = Client::connect().await?;
            match action.as_str() {
                "on" => {
                    if let Some(interval) = interval {
                        client.set_auto_switch_interval(interval).await?;
                    }
                    client.set_auto_switch(true).await?;
                }
                "off" => {
                    client.set_auto_switch(false).await?;
                }
                "status" => {
                    client.get_auto_switch_status().await?;
                }
                _ => unreachable!(),
            }
        }
        
        Commands::Init { force } => {
            let config_path = config::Config::default_path()
                .ok_or_else(|| anyhow::anyhow!("Could not determine config path"))?;
            
            let config_path = std::path::PathBuf::from(config_path);
            
            if config_path.exists() && !force {
                println!("Config file already exists at: {:?}", config_path);
                println!("Use --force to overwrite");
                return Ok(());
            }
            
            Config::generate_example()?;
            println!("✓ Configuration initialized at: {:?}", config_path);
            println!("\nEdit the file to customize your settings.");
            println!("Then enable the service:");
            println!("  systemctl --user enable --now swww-manager.socket");
        }
        
        Commands::Reload => {
            let mut client = Client::connect().await?;
            client.reload_config().await?;
        }
        
        Commands::Detect => {
            let mut client = Client::connect().await?;
            client.detect_and_switch_profile().await?;
        }
        
        Commands::Monitors { watch } => {
            if watch {
                watch_monitors().await?;
            } else {
                show_monitors().await?;
            }
        }
    }

    Ok(())
}


async fn run_event_monitor() -> Result<()> {
    use crate::hyprland_event::{monitor_events, HyprlandEvent};
    use futures::FutureExt;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::Mutex;
    use tokio::task::JoinHandle;

    let scheduled_task: Arc<Mutex<Option<JoinHandle<()>>>> = Arc::new(Mutex::new(None));
    let debounce_delay = Duration::from_millis(900);
    
    monitor_events(move |event| {
        let scheduled_task = Arc::clone(&scheduled_task);
        let debounce_delay = debounce_delay.clone();
        async move {
            match event {
                HyprlandEvent::MonitorAdded { .. } | HyprlandEvent::MonitorRemoved { .. }=> {
                    let event_type = match event {
                        HyprlandEvent::MonitorAdded { .. } => "added",
                        HyprlandEvent::MonitorRemoved { .. } => "removed",
                        _ => unreachable!(),
                    };
                    info!("Monitor: {} (debouncing)", event_type);

                    if let Some(handle) = scheduled_task.lock().await.take() {
                        handle.abort();
                    }

                    let handle = tokio::spawn(async move {
                        tokio::time::sleep(debounce_delay).await;
                        if let Ok(mut client) = Client::connect().await {
                            if let Err(e) = client.detect_and_switch_profile().await {
                                tracing::warn!("Failed to switch profile after monitor change: {}", e);
                            }
                        }
                    });

                    *scheduled_task.lock().await = Some(handle);
                }
                _ => {}
            }
        }.boxed()
    }).await
}

async fn show_monitors() -> Result<()> {
    use hyprland_ipc::HyprlandIPC;
    
    let ipc = HyprlandIPC::new()?;
    let monitors = ipc.get_monitors().await?;
    
    println!("\nConnected Monitors:");
    println!("{}", "═".repeat(70));
    
    for monitor in monitors {
        println!("\n{} {}", 
            if monitor.focused { "➤" } else { " " },
            monitor.name
        );
        println!("  Description: {}", monitor.description);
        println!("  Resolution:  {}x{} @ {:.2}Hz", 
            monitor.width, monitor.height, monitor.refreshRate);
        println!("  Position:    ({}, {})", monitor.x, monitor.y);
        println!("  Scale:       {:.2}x", monitor.scale);
        println!("  Workspace:   {} (ID: {})", 
            monitor.activeWorkspace.name, monitor.activeWorkspace.id);
        println!("  Status:      {}", 
            if monitor.dpmsStatus { "On" } else { "Off" });
    }
    
    println!();
    Ok(())
}

async fn watch_monitors() -> Result<()> {
    use hyprland_ipc::HyprlandIPC;
    
    println!("Watching for monitor changes... (Press Ctrl+C to exit)\n");
    
    let ipc = HyprlandIPC::new()?;
    let mut last_monitors = ipc.get_monitors().await?;
    
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        match ipc.get_monitors().await {
            Ok(current_monitors) => {
                if current_monitors.len() != last_monitors.len() {
                    println!("\n[{}] Monitor count changed: {} → {}", 
                        chrono::Local::now().format("%H:%M:%S"),
                        last_monitors.len(),
                        current_monitors.len()
                    );
                    
                    for monitor in &current_monitors {
                        if !last_monitors.iter().any(|m| m.name == monitor.name) {
                            println!("  + Added: {} ({})", monitor.name, monitor.description);
                        }
                    }
                    
                    for monitor in &last_monitors {
                        if !current_monitors.iter().any(|m| m.name == monitor.name) {
                            println!("  - Removed: {} ({})", monitor.name, monitor.description);
                        }
                    }
                    
                    last_monitors = current_monitors;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get monitors: {}", e);
            }
        }
    }
}
