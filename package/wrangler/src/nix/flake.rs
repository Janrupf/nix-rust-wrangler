use crate::error::FlakeEvalError;
use crate::nix::NixCommand;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Output;

#[derive(Debug, Clone)]
pub struct NixFlake {
    flake_path: PathBuf,
    flake_dir: PathBuf,
}

impl NixFlake {
    /// Search the tree upwards from the process working directory to find a flake.nix file.
    /// If NIX_RUST_WRANGLER_FLAKE_PATH is set, use that as the flake path.
    pub fn find_automatically() -> Option<Self> {
        if let Some(flake_path) = std::env::var_os("NIX_RUST_WRANGLER_FLAKE_PATH") {
            let flake_path = PathBuf::from(flake_path);
            let flake_dir = flake_path
                .parent()
                .unwrap_or_else(|| Path::new("/"))
                .to_path_buf();

            return Some(Self {
                flake_path,
                flake_dir,
            });
        }

        let pwd = std::env::current_dir()
            .map_err(|err| {
                tracing::warn!("Failed to determine current working directory: {}", err);
                err
            })
            .ok()?;

        if let Some(flake) = Self::find(&pwd) {
            return Some(flake);
        }

        // Last resort: search upwards of our own executable
        let exe = PathBuf::from(std::env::args_os().next()?);
        tracing::debug!("Attempting to find flake relative to own executable: {}", exe.display());
        
        let exe_dir = exe.parent()?;
        Self::find(exe_dir)
    }

    /// Search the tree upwards from the given path to find a flake.nix file.
    pub fn find(start: &Path) -> Option<Self> {
        let mut current = start;
        loop {
            let flake_path = current.join("flake.nix");
            tracing::trace!("Checking for flake.nix at {:?}", flake_path);

            if flake_path.is_file() {
                tracing::debug!("Found flake.nix at {:?}", flake_path);
                return Some(Self {
                    flake_path,
                    flake_dir: current.to_path_buf(),
                });
            }

            current = current.parent()?;
        }
    }

    pub fn path(&self) -> &Path {
        &self.flake_path
    }

    pub fn dir(&self) -> &Path {
        &self.flake_dir
    }

    /// Apply a nix expression to the flake and return the result as JSON.
    pub fn apply_expr_json<T: serde::de::DeserializeOwned>(
        &self,
        nix_command: &NixCommand,
        attr: impl AsRef<str>,
        expr: impl AsRef<str>,
    ) -> Result<T, FlakeEvalError> {
        let installable_expression = self.installable(attr);

        tracing::trace!("Evaluating flake expression: {:?}", installable_expression);

        let eval_output = nix_command
            .new_command()
            .args(["eval", "--json"])
            .arg(installable_expression)
            .args(["--apply", expr.as_ref()])
            .output()?;

        let eval_output = Self::handle_nix_output(eval_output)?;
        serde_json::from_slice::<T>(&eval_output).map_err(Into::into)
    }

    pub fn build(
        &self,
        nix_command: &NixCommand,
        attr: impl AsRef<str>,
    ) -> Result<Vec<FlakeBuildOutput>, FlakeEvalError> {
        let installable_expression = self.installable(attr);
        tracing::trace!("Building flake expression: {:?}", installable_expression);

        let build_output = nix_command
            .new_command()
            .args(["build", "--no-link", "--json"])
            .arg(installable_expression)
            .output()?;

        let build_output = Self::handle_nix_output(build_output)?;
        serde_json::from_slice::<Vec<FlakeBuildOutput>>(&build_output).map_err(Into::into)
    }

    fn handle_nix_output(output: Output) -> Result<Vec<u8>, FlakeEvalError> {
        tracing::debug!("Nix exited with status: {:?}", output.status);
        tracing::debug!("Nix stdout: {}", String::from_utf8_lossy(&output.stdout));
        tracing::debug!("Nix stderr: {}", String::from_utf8_lossy(&output.stderr));

        if output.status.code() != Some(0) {
            return Err(FlakeEvalError::EvalFailed {
                status: output.status,
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            });
        }

        Ok(output.stdout)
    }

    pub fn installable(&self, attr: impl AsRef<str>) -> OsString {
        let mut installable_expression = OsString::from(self.flake_dir.clone());
        installable_expression.push("#");
        installable_expression.push(attr.as_ref());
        installable_expression
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlakeBuildOutput {
    pub drv_path: PathBuf,
    pub outputs: HashMap<String, PathBuf>,
}
