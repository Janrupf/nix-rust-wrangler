use crate::error::Error;
use std::ffi::OsString;
use std::path::Path;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum InvokedTool {
    Rustc,
    RustDoc,
    Cargo,
    RustLLDB,
    RustGDB,
    RustGDBGUI,
    Rls,
    CargoClippy,
    ClippyDriver,
    CargoMiri,
    RustAnalayzer,
    RustFmt,
    CargoFmt,
    Rustup,
    NixRustWrangler,
    NixDevelopProxy,
    Other(OsString),
}

impl InvokedTool {
    pub fn to_executable_name(&self) -> OsString {
        match self {
            Self::Rustc => "rustc".into(),
            Self::RustDoc => "rustdoc".into(),
            Self::Cargo => "cargo".into(),
            Self::RustLLDB => "rust-lldb".into(),
            Self::RustGDB => "rust-gdb".into(),
            Self::RustGDBGUI => "rust-gdbgui".into(),
            Self::Rls => "rls".into(),
            Self::CargoClippy => "cargo-clippy".into(),
            Self::ClippyDriver => "clippy-driver".into(),
            Self::CargoMiri => "cargo-miri".into(),
            Self::RustAnalayzer => "rust-analyzer".into(),
            Self::RustFmt => "rustfmt".into(),
            Self::CargoFmt => "cargo-fmt".into(),
            Self::Rustup => "rustup".into(),
            Self::NixRustWrangler => "nix-rust-wrangler".into(),
            Self::NixDevelopProxy => "nix-develop-proxy".into(),
            Self::Other(o) => o.clone(),
        }
    }

    pub fn to_name(&self) -> String {
        match self {
            Self::Rustc => "rustc".into(),
            Self::RustDoc => "rustdoc".into(),
            Self::Cargo => "cargo".into(),
            Self::RustLLDB => "rust-lldb".into(),
            Self::RustGDB => "rust-gdb".into(),
            Self::RustGDBGUI => "rust-gdbgui".into(),
            Self::Rls => "rls".into(),
            Self::CargoClippy => "cargo-clippy".into(),
            Self::ClippyDriver => "clippy-driver".into(),
            Self::CargoMiri => "cargo-miri".into(),
            Self::RustAnalayzer => "rust-analyzer".into(),
            Self::RustFmt => "rustfmt".into(),
            Self::CargoFmt => "cargo-fmt".into(),
            Self::Rustup => "rustup".into(),
            Self::NixRustWrangler => "nix-rust-wrangler".into(),
            Self::NixDevelopProxy => "nix-develop-proxy".into(),
            Self::Other(o) => o.to_string_lossy().into_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Invocation {
    pub tool: InvokedTool,
    pub toolchain_override: ToolchainOverride,
    pub remaining_args: Vec<OsString>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ToolchainOverride {
    None,
    FromEnv(String),
    FromArg(String),
}

impl ToolchainOverride {
    pub fn as_override_name(&self) -> Option<&str> {
        match self {
            Self::None => None,
            Self::FromEnv(name) | Self::FromArg(name) => Some(name),
        }
    }
}

impl Invocation {
    /// Derive the invocation from the command line arguments and environment
    /// variables.
    pub fn derive() -> Result<Self, Error> {
        let mut args = std::env::args_os();
        let self_arg = args.next().ok_or(Error::MissingArgv0)?;

        let mut tool = Self::self_arg_to_tool(self_arg)?;

        let mut remaining_args = Vec::new();

        // Try to determine the toolchain override and/or tool invocation next.
        let mut toolchain_override = ToolchainOverride::None;
        let next = args.next();
        if let Some(next) = next {
            if let Some(toolchain_name) = next.to_str().and_then(|s| s.strip_prefix('+')) {
                toolchain_override = ToolchainOverride::FromArg(toolchain_name.to_string());
            } else if tool == InvokedTool::NixRustWrangler {
                tool = Self::self_arg_to_tool(next)?;
            } else {
                remaining_args.push(next);
            }
        }

        if matches!(toolchain_override, ToolchainOverride::FromArg(_))
            && tool == InvokedTool::NixRustWrangler
        {
            // Next arg is the tool to invoke
            tool = args
                .next()
                .ok_or(Error::MissingTool)
                .and_then(Self::self_arg_to_tool)?;
        }

        remaining_args.extend(args);

        if toolchain_override == ToolchainOverride::None {
            toolchain_override = Self::toolchain_override_from_env("NIX_RUST_WRANGLER_TOOLCHAIN")?;
        }
        if toolchain_override == ToolchainOverride::None {
            toolchain_override = Self::toolchain_override_from_env("RUSTUP_TOOLCHAIN")?;
        }

        Ok(Self {
            tool,
            toolchain_override,
            remaining_args,
        })
    }

    fn self_arg_to_tool(self_arg: OsString) -> Result<InvokedTool, Error> {
        let path = Path::new(&self_arg);

        let file_name = path.file_stem().ok_or(Error::InvalidToolName)?;
        if let Some(tool_name) = file_name.to_str() {
            Ok(match tool_name {
                "rustc" => InvokedTool::Rustc,
                "rustdoc" => InvokedTool::RustDoc,
                "cargo" => InvokedTool::Cargo,
                "rust-lldb" => InvokedTool::RustLLDB,
                "rust-gdb" => InvokedTool::RustGDB,
                "rust-gdbgui" => InvokedTool::RustGDBGUI,
                "rls" => InvokedTool::Rls,
                "cargo-clippy" => InvokedTool::CargoClippy,
                "clippy-driver" => InvokedTool::ClippyDriver,
                "cargo-miri" => InvokedTool::CargoMiri,
                "rust-analyzer" => InvokedTool::RustAnalayzer,
                "rustfmt" => InvokedTool::RustFmt,
                "cargo-fmt" => InvokedTool::CargoFmt,
                "rustup" => InvokedTool::Rustup,
                "nix-rust-wrangler" => InvokedTool::NixRustWrangler,
                "nix-develop-proxy" => InvokedTool::NixDevelopProxy,
                _ => InvokedTool::Other(tool_name.into()),
            })
        } else {
            Ok(InvokedTool::Other(file_name.into()))
        }
    }

    fn toolchain_override_from_env(env_name: impl AsRef<str>) -> Result<ToolchainOverride, Error> {
        let env_name = env_name.as_ref();

        match std::env::var(env_name) {
            Ok(toolchain_name) => Ok(ToolchainOverride::FromEnv(toolchain_name)),
            Err(std::env::VarError::NotPresent) => Ok(ToolchainOverride::None),
            Err(std::env::VarError::NotUnicode(_)) => Err(Error::ToolchainEnvNameNotUnicode),
        }
    }
}
