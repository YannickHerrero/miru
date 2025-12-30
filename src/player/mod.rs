use std::process::{Command, Stdio};

use crate::config::PlayerConfig;
use crate::error::PlayerError;

/// Player wrapper for launching external media players
pub struct Player {
    config: PlayerConfig,
}

impl Player {
    pub fn new(config: PlayerConfig) -> Self {
        Self { config }
    }

    /// Check if the configured player is available in PATH
    pub fn is_available(&self) -> bool {
        which::which(&self.config.command).is_ok()
    }

    /// Play a URL with the configured player
    pub fn play(&self, url: &str) -> Result<(), PlayerError> {
        if !self.is_available() {
            return Err(PlayerError::NotFound(self.config.command.clone()));
        }

        let mut cmd = Command::new(&self.config.command);

        // Add configured arguments
        for arg in &self.config.args {
            cmd.arg(arg);
        }

        // Add the URL
        cmd.arg(url);

        // Inherit stdio so player can interact with terminal
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        tracing::info!("Launching player: {} {}", self.config.command, url);

        let status = cmd
            .status()
            .map_err(|e| PlayerError::LaunchFailed(e.to_string()))?;

        if !status.success() {
            if let Some(code) = status.code() {
                return Err(PlayerError::ExitError(format!("Exit code: {}", code)));
            }
        }

        Ok(())
    }

    /// Get the player command name
    #[allow(dead_code)]
    pub fn command(&self) -> &str {
        &self.config.command
    }
}
