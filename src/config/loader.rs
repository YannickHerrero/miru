use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::config::Config;
use crate::error::ConfigError;

/// Get the config file path (~/.config/miru/config.toml)
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("miru")
        .join("config.toml")
}

/// Load config from the default path
pub fn load_config() -> Result<Config, ConfigError> {
    let path = config_path();

    if !path.exists() {
        return Err(ConfigError::NotFound);
    }

    let content = fs::read_to_string(&path)?;
    let config: Config =
        toml::from_str(&content).map_err(|e| ConfigError::Invalid(e.to_string()))?;

    if !config.has_api_key() {
        return Err(ConfigError::MissingApiKey);
    }

    Ok(config)
}

/// Save config to the default path with secure permissions
pub fn save_config(config: &Config) -> Result<(), ConfigError> {
    let path = config_path();

    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(config)
        .map_err(|e| ConfigError::SaveFailed(e.to_string()))?;

    fs::write(&path, content)?;

    // Set secure permissions (0600) on Unix
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }

    Ok(())
}

/// Check if config file exists
#[allow(dead_code)]
pub fn config_exists() -> bool {
    config_path().exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_path() {
        let path = config_path();
        assert!(path.ends_with("miru/config.toml"));
    }
}
