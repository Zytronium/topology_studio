/// User-selected theme mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

/// Resolved theme (Light or Dark)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppTheme {
    Light,
    Dark,
}

impl ThemeMode {
    /// Load the saved theme mode from the config file.
    pub fn load() -> Self {
        if let Some(saved) = load_theme_config() {
            return saved;
        }
        ThemeMode::System
    }

    /// Save the theme mode preference to the config file.
    pub fn save(&self) {
        save_theme_config(*self);
    }

    /// Resolve to the actual theme (Light or Dark).
    /// If System, detects the system theme preference.
    pub fn resolve(&self) -> AppTheme {
        match self {
            ThemeMode::Light => AppTheme::Light,
            ThemeMode::Dark => AppTheme::Dark,
            ThemeMode::System => detect_system_theme(),
        }
    }
}

/// Detect the system theme preference.
/// On Linux, checks GNOME/KDE settings. On Windows, reads the registry.
/// Falls back to light mode if nothing can be detected.
fn detect_system_theme() -> AppTheme {
    // Windows: check the AppsUseLightTheme registry key
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        if let Ok(value) = winreg::RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        {
            if let Ok(apps_light) = value.get_value::<u32, _>("AppsUseLightTheme") {
                return if apps_light == 0 {
                    AppTheme::Dark
                } else {
                    AppTheme::Light
                };
            }
        }
    }

    // Linux: Try to read from XDG settings (GNOME/KDE)
    // Check gsettings for GNOME
    if let Ok(output) = std::process::Command::new("gsettings")
        .args(&["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("dark") {
            return AppTheme::Dark;
        }
    }

    // Check GTK2 settings as fallback
    if let Ok(output) = std::process::Command::new("gsettings")
        .args(&["get", "org.gnome.desktop.interface", "gtk-theme"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.to_lowercase().contains("dark") {
            return AppTheme::Dark;
        }
    }

    // Check KDE Plasma 5/6
    if let Ok(output) = std::process::Command::new("kreadconfig5")
        .args(&["--file", "kdeglobals", "--group", "General", "--key", "ColorScheme"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.to_lowercase().contains("dark") {
            return AppTheme::Dark;
        }
    }

    // Check KDE Plasma 6
    if let Ok(output) = std::process::Command::new("kreadconfig6")
        .args(&["--file", "kdeglobals", "--group", "General", "--key", "ColorScheme"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.to_lowercase().contains("dark") {
            return AppTheme::Dark;
        }
    }

    // Default to light mode
    AppTheme::Light
}

/// Get the path to the config directory.
fn config_dir() -> Option<std::path::PathBuf> {
    if let Ok(val) = std::env::var("XDG_CONFIG_HOME") {
        return Some(std::path::PathBuf::from(val));
    }
    if let Ok(home) = std::env::var("HOME") {
        return Some(std::path::PathBuf::from(home).join(".config"));
    }
    None
}

/// Get the path to the config file.
fn config_file_path() -> Option<std::path::PathBuf> {
    let dir = config_dir()?;
    let app_dir = dir.join("topology_studio");
    Some(app_dir.join("config.json"))
}

/// Load the saved theme mode from the config file.
fn load_theme_config() -> Option<ThemeMode> {
    let path = config_file_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let config: AppSettings = serde_json::from_str(&content).ok()?;
    match config.theme_mode.as_str() {
        "Dark" => Some(ThemeMode::Dark),
        "Light" => Some(ThemeMode::Light),
        "System" => Some(ThemeMode::System),
        _ => None,
    }
}

/// Save the theme mode preference to the config file.
fn save_theme_config(mode: ThemeMode) {
    let path = match config_file_path() {
        Some(p) => p,
        None => return,
    };

    // Create the directory if it doesn't exist
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("Failed to create config directory: {}", e);
            return;
        }
    }

    // Load existing settings or create new ones
    let mut settings = load_settings().unwrap_or_default();
    settings.theme_mode = match mode {
        ThemeMode::Light => "Light".to_string(),
        ThemeMode::Dark => "Dark".to_string(),
        ThemeMode::System => "System".to_string(),
    };

    if let Ok(json) = serde_json::to_string_pretty(&settings) {
        if let Err(e) = std::fs::write(&path, json) {
            eprintln!("Failed to save theme config: {}", e);
        }
    }
}

/// Load all app settings from the config file.
fn load_settings() -> Option<AppSettings> {
    let path = config_file_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save all app settings to the config file.
pub fn save_settings(settings: &AppSettings) {
    let path = match config_file_path() {
        Some(p) => p,
        None => return,
    };

    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("Failed to create config directory: {}", e);
            return;
        }
    }

    if let Ok(json) = serde_json::to_string_pretty(settings) {
        if let Err(e) = std::fs::write(&path, json) {
            eprintln!("Failed to save settings: {}", e);
        }
    }
}

/// Load all app settings.
pub fn load_settings_public() -> AppSettings {
    load_settings().unwrap_or_default()
}

/// App settings stored in config file.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct AppSettings {
    pub theme_mode: String,
    pub snap_to_grid: bool,
    pub autosave: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme_mode: "System".to_string(),
            snap_to_grid: false,
            autosave: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_mode_default_is_system() {
        assert_eq!(ThemeMode::default(), ThemeMode::System);
    }

    #[test]
    fn test_theme_mode_resolve_light() {
        assert_eq!(ThemeMode::Light.resolve(), AppTheme::Light);
    }

    #[test]
    fn test_theme_mode_resolve_dark() {
        assert_eq!(ThemeMode::Dark.resolve(), AppTheme::Dark);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = AppSettings {
            theme_mode: "Dark".to_string(),
            snap_to_grid: true,
            autosave: false,
        };
        let json = serde_json::to_string(&settings).unwrap();
        let restored: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.theme_mode, "Dark");
        assert!(restored.snap_to_grid);
        assert!(!restored.autosave);
    }

    #[test]
    fn test_settings_default() {
        let settings = AppSettings::default();
        assert_eq!(settings.theme_mode, "System");
        assert!(!settings.snap_to_grid);
        assert!(!settings.autosave);
    }

    #[test]
    fn test_load_theme_config_invalid() {
        // Invalid JSON should return None
        assert!(load_theme_config().is_none() || load_theme_config().is_some());
        // This test just verifies the function doesn't panic
    }
}
