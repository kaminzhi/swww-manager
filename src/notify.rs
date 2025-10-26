use anyhow::Result;
use std::process::Command;

pub fn send(title: &str, message: &str) -> Result<()> {
    let notification = format!("rgb(88ccff) {}: {}", title, message);
    
    let _ = Command::new("hyprctl")
        .args(&["notify", "3", "5000", &notification])
        .output();
    
    Ok(())
}

pub fn send_error(message: &str) -> Result<()> {
    let notification = format!("rgb(ff8888) Error: {}", message);
    
    let _ = Command::new("hyprctl")
        .args(&["notify", "4", "8000", &notification])
        .output();
    
    Ok(())
}

pub fn send_success(message: &str) -> Result<()> {
    let notification = format!("rgb(88ff88) Success: {}", message);
    
    let _ = Command::new("hyprctl")
        .args(&["notify", "1", "3000", &notification])
        .output();
    
    Ok(())
}
