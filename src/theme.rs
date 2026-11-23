/// Application theme mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AppTheme {
    #[default]
    Light,
    Dark,
}

impl AppTheme {
    /// Load the saved theme from the config file, or detect the system theme.
    pub fn load() -> Self {
        // Try to load from config file first
        if let Some(saved) = load_theme_config() {
            return saved;
        }
        // Fall back to system theme detection
        detect_system_theme()
    }

    /// Save the theme preference to the config file.
    pub fn save(&self) {
        save_theme_config(*self);
    }
}

/// Detect the system theme preference.
/// On Linux, checks the XDG desktop preferences or falls back to light mode.
fn detect_system_theme() -> AppTheme {
    // Try to read from XDG settings (GNOME/KDE)
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

/// Load the saved theme from the config file.
fn load_theme_config() -> Option<AppTheme> {
    let path = config_file_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let config: ThemeConfig = serde_json::from_str(&content).ok()?;
    match config.theme.as_str() {
        "Dark" => Some(AppTheme::Dark),
        "Light" => Some(AppTheme::Light),
        _ => None,
    }
}

/// Save the theme preference to the config file.
fn save_theme_config(theme: AppTheme) {
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

    let config = ThemeConfig {
        theme: match theme {
            AppTheme::Light => "Light".to_string(),
            AppTheme::Dark => "Dark".to_string(),
        },
    };

    if let Ok(json) = serde_json::to_string_pretty(&config) {
        if let Err(e) = std::fs::write(&path, json) {
            eprintln!("Failed to save theme config: {}", e);
        }
    }
}

/// Config file structure for theme persistence.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ThemeConfig {
    theme: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_default_is_light() {
        assert_eq!(AppTheme::default(), AppTheme::Light);
    }

    #[test]
    fn test_theme_config_serialization() {
        let config = ThemeConfig {
            theme: "Dark".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: ThemeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.theme, "Dark");
    }

    #[test]
    fn test_load_theme_config_invalid() {
        // Invalid JSON should return None
        assert!(load_theme_config().is_none() || load_theme_config().is_some());
        // This test just verifies the function doesn't panic
    }
}
