use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use dirs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_tracking_interval")]
    pub tracking_interval_ms: u64,
    
    #[serde(default = "default_save_interval")]
    pub save_interval_ms: u64,
    
    #[serde(default = "default_deep_focus_threshold")]
    pub deep_focus_threshold_minutes: u64,
    
    #[serde(default)]
    pub focus_apps: Vec<String>,
    
    #[serde(default)]
    pub ignored_apps: Vec<String>,
    
    #[serde(default)]
    pub focus_sites: Vec<String>,
    
    #[serde(default)]
    pub ignored_sites: Vec<String>,
    
    #[serde(default = "default_database_path")]
    pub database_path: Option<String>,
    

    
    #[serde(default = "default_first_run")]
    pub first_run: bool,
}



impl Default for Config {
    fn default() -> Self {
        Self {
            tracking_interval_ms: default_tracking_interval(),
            save_interval_ms: default_save_interval(),
            deep_focus_threshold_minutes: default_deep_focus_threshold(),
            focus_apps: Vec::new(),
            ignored_apps: Vec::new(),
            focus_sites: Vec::new(),
            ignored_sites: Vec::new(),
            database_path: default_database_path(),

            first_run: default_first_run(),
        }
    }
}



fn default_tracking_interval() -> u64 { 1000 }
fn default_save_interval() -> u64 { 30000 }
fn default_deep_focus_threshold() -> u64 { 30 }

fn default_first_run() -> bool { true }

fn default_database_path() -> Option<String> {
    Some("focusdebt.db".to_string())
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            // Create default config
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        
        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir()
            .ok_or("Could not find config directory")?;
        
        Ok(config_dir.join("focusdebt").join("config.toml"))
    }

    pub fn add_focus_app(&mut self, app_name: String) {
        if !self.focus_apps.contains(&app_name) {
            self.focus_apps.push(app_name);
        }
    }

    pub fn remove_focus_app(&mut self, app_name: &str) {
        self.focus_apps.retain(|app| app != app_name);
    }

    pub fn add_ignored_app(&mut self, app_name: String) {
        if !self.ignored_apps.contains(&app_name) {
            self.ignored_apps.push(app_name);
        }
    }

    pub fn remove_ignored_app(&mut self, app_name: &str) {
        self.ignored_apps.retain(|app| app != app_name);
    }

    pub fn is_focus_app(&self, app_name: &str) -> bool {
        self.focus_apps.contains(&app_name.to_string())
    }

    pub fn is_ignored_app(&self, app_name: &str) -> bool {
        self.ignored_apps.contains(&app_name.to_string())
    }

    pub fn get_database_path(&self) -> PathBuf {
        if let Some(ref path) = self.database_path {
            PathBuf::from(path)
        } else {
            // Fallback to default location
            dirs::data_dir()
                .map(|dir| dir.join("focusdebt").join("focusdebt.db"))
                .unwrap_or_else(|| PathBuf::from("focusdebt.db"))
        }
    }

    pub fn mark_first_run_complete(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.first_run = false;
        self.save()
    }

    pub fn add_focus_site(&mut self, site: String) {
        if !self.focus_sites.contains(&site) {
            self.focus_sites.push(site);
        }
    }

    pub fn remove_focus_site(&mut self, site: &str) {
        self.focus_sites.retain(|s| s != site);
    }

    pub fn add_ignored_site(&mut self, site: String) {
        if !self.ignored_sites.contains(&site) {
            self.ignored_sites.push(site);
        }
    }

    pub fn remove_ignored_site(&mut self, site: &str) {
        self.ignored_sites.retain(|s| s != site);
    }

    pub fn is_focus_site(&self, site: &str) -> bool {
        self.focus_sites.contains(&site.to_string())
    }

    pub fn is_ignored_site(&self, site: &str) -> bool {
        self.ignored_sites.contains(&site.to_string())
    }
} 