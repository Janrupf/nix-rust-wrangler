pub mod flake;
pub mod config;
pub mod proxy;

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct NixCommand {
    executable: PathBuf,
    is_usable: bool,
    flakes_enabled: bool,
}

impl NixCommand {
    pub fn find() -> Option<Self> {
        let executable = crate::util::find_executable_in_path("nix")?;

        tracing::trace!(
            "Running '{:?} config show experimental-features'",
            executable
        );

        // Invoke "nix config show experimental-features" to determine if flakes are enabled
        // - this command may outright fail, if nix-command is not enabled, which also
        //  means that flakes are not enabled (or at least not available)
        let nix_config_output = match Command::new(&executable)
            .args(&["config", "show", "experimental-features"])
            .output()
        {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(
                    "Failed to invoke 'nix config show experimental-features': {}",
                    err
                );
                return Some(Self {
                    executable,
                    is_usable: false,
                    flakes_enabled: false,
                });
            }
        };

        tracing::debug!("'nix config show experimental-features' output:");
        tracing::debug!("- status: {:?}", nix_config_output.status);
        tracing::debug!(
            "- stdout: {}",
            String::from_utf8_lossy(&nix_config_output.stdout)
        );
        tracing::debug!(
            "- stderr: {}",
            String::from_utf8_lossy(&nix_config_output.stderr)
        );

        if !matches!(nix_config_output.status.code(), Some(0)) {
            // Not enabled or not available
            return Some(Self {
                executable,
                is_usable: false,
                flakes_enabled: false,
            });
        }

        let features = match String::from_utf8(nix_config_output.stdout) {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(
                    "Failed to parse 'nix config show experimental-features' output: {}",
                    err
                );
                return Some(Self {
                    executable,
                    is_usable: false,
                    flakes_enabled: false,
                });
            }
        };

        let mut flakes_enabled = false;
        for feature in features.trim().split(' ') {
            if feature == "flakes" {
                flakes_enabled = true;
                break;
            }
        }

        Some(Self {
            executable,
            is_usable: true,
            flakes_enabled,
        })
    }

    pub fn is_usable(&self) -> bool {
        self.is_usable
    }

    pub fn flakes_enabled(&self) -> bool {
        self.flakes_enabled
    }

    pub fn new_command(&self) -> Command {
        Command::new(&self.executable)
    }
    
    pub fn executable(&self) -> &Path {
        &self.executable
    }
}
