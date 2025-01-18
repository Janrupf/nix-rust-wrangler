use crate::error::{Error, FlakeEvalError};
use crate::invocation::{Invocation, InvokedTool, ToolchainOverride};
use crate::invoker::ToolInvoker;
use crate::nix::flake::NixFlake;
use crate::nix::NixCommand;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt::Display;
use std::ops::Deref;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlakeInspection {
    // Quick configuration shells:
    pub default_dev_shell: Option<FlakeValueType>,
    pub rust_wrangler_dev_shell: Option<FlakeValueType>,

    // More complex config
    pub config: Option<FlakeEmbeddedConfigAttr>,
}

impl FlakeInspection {
    pub const APPLY_EXPR: &'static str = include_str!("./inspect-flake.nix");

    /// Attempt to create an invoker which invokes the tool using the flake's configuration.
    pub fn make_invoker(
        &self,
        nix_command: &NixCommand,
        flake: &NixFlake,
        invocation: &Invocation,
    ) -> Option<Result<ToolInvoker, Error>> {
        let mut toolchain_for_cargo_fallback = None;

        if let Some(config) = &self.config {
            if config.ignore {
                return None;
            }

            if let Some(name) = invocation.toolchain_override.as_override_name() {
                if let Some(toolchain) = config.toolchains.get(name) {
                    if let FlakeValueType::Derivation = toolchain {
                        let build_result = self.build_toolchain(
                            invocation,
                            nix_command,
                            flake,
                            config,
                            format!("toolchains.{}", name),
                        );

                        match build_result {
                            Ok(v) => return Some(Ok(v)),
                            Err(Error::ToolchainDoesNotProvideTool { path, .. })
                                if invocation.tool == InvokedTool::Cargo =>
                            {
                                // This can happen, custom toolchains don't necessarily provide cargo
                                tracing::debug!(
                                    "Toolchain selected via override does not provide cargo"
                                );

                                toolchain_for_cargo_fallback = Some(path);
                            }
                            Err(err) => return Some(Err(err)),
                        }
                    } else {
                        tracing::warn!(
                            "Toolchain attribute {} is not a derivation, ignoring",
                            name
                        );
                    }
                }

                if toolchain_for_cargo_fallback.is_none() {
                    tracing::warn!(
                        "Flake does not provide toolchain override '{}', continuing search outside of the flake",
                        name
                    );
                    return None;
                }
            }

            // Not overwritten, use the default toolchain
            let toolchain_build_result = config
                .toolchain
                .as_ref()
                .filter(|v| {
                    if let FlakeValueType::Derivation = **v {
                        true
                    } else {
                        tracing::warn!("Toolchain attribute is not a derivation, ignoring");
                        false
                    }
                })
                .map(|_| self.build_toolchain(invocation, nix_command, flake, config, "toolchain"));

            match toolchain_build_result {
                None => {
                    if !config.toolchains.is_empty() {
                        tracing::warn!(
                            "No default toolchain found, but overrides are defined. \
                            Did you mean to define a default toolchain? \
                            Continuing search outside of the flake."
                        );
                    }
                }
                Some(Ok(v)) => return Some(Ok(v)),
                Some(Err(Error::ToolchainDoesNotProvideTool { path, .. }))
                    if invocation.tool == InvokedTool::Cargo =>
                {
                    toolchain_for_cargo_fallback = Some(path);

                    // This can happen, custom toolchains don't necessarily provide cargo
                    tracing::debug!("Default toolchain does not provide cargo");
                }
                Some(Err(err)) => return Some(Err(err)),
            }
        }

        // No special toolchain config, attempt to use dev shells
        self.rust_wrangler_dev_shell
            .as_ref()
            .filter(|v| **v == FlakeValueType::Derivation)
            .map(|_| {
                self.create_develop_proxy(
                    nix_command,
                    flake,
                    Some("rustWrangler"),
                    &invocation.toolchain_override,
                    toolchain_for_cargo_fallback.clone(),
                )
            })
            .or_else(|| {
                self.default_dev_shell
                    .as_ref()
                    .filter(|v| **v == FlakeValueType::Derivation)
                    .map(|_| {
                        self.create_develop_proxy(
                            nix_command,
                            flake,
                            None::<&str>,
                            &invocation.toolchain_override,
                            toolchain_for_cargo_fallback,
                        )
                    })
            })
            .map(|cmd| cmd.map(|cmd| ToolInvoker::from_command_proxy(cmd, invocation)))
    }

    /// Create a command which re-executes ourselves in a nix-develop shell.
    fn create_develop_proxy(
        &self,
        nix_command: &NixCommand,
        flake: &NixFlake,
        shell_name: Option<impl AsRef<str>>,
        toolchain_override: &ToolchainOverride,
        toolchain_for_cargo_fallback: Option<PathBuf>,
    ) -> Result<Command, Error> {
        let own_executable =
            std::env::current_exe().map_err(Error::UnableToDetermineOwnExecutable)?;

        let mut cmd = nix_command.new_command();
        cmd.arg("develop");

        if let Some(shell_name) = shell_name {
            let mut path = OsString::from(flake.dir());
            path.push("#");
            path.push(shell_name.as_ref());

            cmd.arg(path);
        } else {
            cmd.arg(flake.dir());
        }

        cmd.arg("--command");
        cmd.arg(own_executable);
        cmd.arg("nix-develop-proxy");

        if let Some(toolchain_for_cargo_fallback) = toolchain_for_cargo_fallback {
            cmd.env(
                "NIX_RUST_WRANGLER_TOOLCHAIN_FALLBACK",
                toolchain_for_cargo_fallback,
            );
        }
        
        match toolchain_override {
            ToolchainOverride::FromArg(name) | ToolchainOverride::FromEnv(name) => {
                cmd.env("RUSTUP_TOOLCHAIN", name);
                cmd.env("NIX_RUST_WRANGLER_TOOLCHAIN", name);
            }
            ToolchainOverride::None => {}
        }

        cmd.env("NIX_RUST_WRANGLER_INSIDE_NIX_DEVELOP", "1");

        Ok(cmd)
    }

    fn build_toolchain(
        &self,
        invocation: &Invocation,
        nix_command: &NixCommand,
        flake: &NixFlake,
        config: &FlakeEmbeddedConfigAttr,
        toolchain_attr_path: impl Display,
    ) -> Result<ToolInvoker, Error> {
        let build_result = flake.build(
            nix_command,
            format!("{}.{}", config.at, toolchain_attr_path),
        )?;

        // Find the first usable built toolchain derivation
        for output in build_result {
            if let Some(path) = output.outputs.get("out") {
                return ToolInvoker::from_toolchain_dir(path, invocation);
            }
        }

        Err(Error::Flake(FlakeEvalError::MissingToolchainDerivation))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlakeEmbeddedConfigAttr {
    pub at: String,
    pub value: FlakeEmbeddedConfig,
}

impl Deref for FlakeEmbeddedConfigAttr {
    type Target = FlakeEmbeddedConfig;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlakeEmbeddedConfig {
    #[serde(default)]
    pub ignore: bool,

    pub toolchain: Option<FlakeValueType>,

    #[serde(default)]
    pub toolchains: HashMap<String, FlakeValueType>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FlakeValueType {
    Derivation,
    Other(String),
}

impl<'de> Deserialize<'de> for FlakeValueType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;

        match s {
            "derivation" => Ok(Self::Derivation),
            _ => Ok(Self::Other(s.to_string())),
        }
    }
}
