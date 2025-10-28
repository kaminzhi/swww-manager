use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Switch { profile: Option<String> },
    SwitchProfile { name: String },
    DetectAndSwitchProfile,
    ListProfiles,
    GetStatus,
    SetAutoSwitch { enabled: bool },
    Shutdown,
    SetAutoSwitchInterval { interval: u64 },
    ReloadConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Success { message: String },
    Error { message: String },
    ProfileList { profiles: Vec<ProfileInfo> },
    Status { status: StatusInfo },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub name: String,
    pub monitors: Vec<String>,
    pub wallpaper_count: usize,
    pub is_current: bool,
    pub transition: Option<String>,
    pub transition_duration: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusInfo {
    pub auto_switch_interval: Option<u64>,
    pub current_profile: String,
    pub current_wallpaper: Option<String>,
    pub auto_switch_enabled: bool,
    pub monitors: Vec<String>,
    pub uptime_secs: u64,
}
