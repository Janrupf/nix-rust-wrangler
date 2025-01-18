use crate::error::Error;
use crate::invocation::Invocation;
use crate::invoker::external::ExternalInvoker;
use crate::nix::flake::NixFlake;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

mod external;

/// Hand-off implementation for invoking tools.

#[derive(Debug)]
enum Inner {
    Internal,
    External(ExternalInvoker),
}

#[derive(Debug)]
pub struct ToolInvoker {
    inner: Inner,
    used_flake: Option<NixFlake>,
}

impl ToolInvoker {
    /// Create the invoker from the toolchain directory.
    ///
    /// This searches for the tool specified in the invocation inside a bin directory.
    pub fn from_toolchain_dir(
        toolchain_dir: &Path,
        invocation: &Invocation,
    ) -> Result<Self, Error> {
        ExternalInvoker::new_at(
            &toolchain_dir.join("bin"),
            invocation.tool.to_executable_name(),
        )
        .map(|invoker| Self::from_external_invoker(invoker, Some(toolchain_dir)))
        .ok_or_else(|| Error::ToolchainDoesNotProvideTool {
            path: toolchain_dir.to_path_buf(),
            tool: invocation.tool.to_name(),
        })
    }

    pub fn from_tool_and_toolchain_dir(tool_exe: &Path, toolchain_dir: &Path) -> Self {
        Self::from_external_invoker(
            ExternalInvoker::from_executable(tool_exe),
            Some(toolchain_dir),
        )
    }

    pub fn from_executable(executable: &Path) -> Self {
        Self::from_external_invoker(ExternalInvoker::from_executable(executable), None)
    }

    fn from_external_invoker(mut invoker: ExternalInvoker, toolchain_dir: Option<&Path>) -> Self {
        invoker.configure_command(|cmd| {
            Self::configure_command_for_toolchain(toolchain_dir, cmd);
            cmd.env("RUST_RECURSION_COUNT", "0");
        });
        Self::new(Inner::External(invoker))
    }

    /// Set up a command environment for running a toolchain.
    ///
    /// This is somewhat copied from rustup, but does a bit less setup. Notably PATH is not
    /// modified, as we want to keep the environment as reproducible as possible.
    pub fn configure_command_for_toolchain(toolchain_dir: Option<&Path>, command: &mut Command) {
        // This section is shamelessly copied from rustup's source code.
        // https://github.com/rust-lang/rustup/blob/525f0d54d8e22deb428661f2b86df3e6541cae0f/src/toolchain.rs#L178

        if let Some(toolchain_dir) = toolchain_dir {
            #[cfg(not(target_os = "macos"))]
            const LIBRARY_PATH_VAR: &str = "LD_LIBRARY_PATH";

            #[cfg(target_os = "macos")]
            const LIBRARY_PATH_VAR: &str = "DYLD_FALLBACK_LIBRARY_PATH";

            #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
            let mut new_library_paths = vec![toolchain_dir.join("lib")];

            #[cfg(target_os = "macos")]
            if std::env::var_os(LIBRARY_PATH_VAR)
                .filter(|v| v.len() > 0)
                .is_none()
            {
                if let Some(home) = std::env::var_os("HOME") {
                    new_library_paths.push(home.join("lib"));
                }
                new_library_paths.push(std::path::PathBuf::from("/usr/local/lib"));
                new_library_paths.push(std::path::PathBuf::from("/usr/lib"));
            }

            // Note: we very explicitly skip adding CARGO_HOME to PATH, as this would
            // be questionable for reproducibility.

            command.env(
                LIBRARY_PATH_VAR,
                crate::util::prepend_paths(std::env::var_os(LIBRARY_PATH_VAR), &new_library_paths),
            );
        }

        crate::util::u32_from_env("RUST_RECURSION_COUNT")
            .checked_add(1)
            .map(|v| command.env("RUST_RECURSION_COUNT", v.to_string()));
    }

    /// Create the invoker based on invoking another command.
    pub fn from_command_proxy(mut partial_command: Command, invocation: &Invocation) -> Self {
        partial_command.arg(invocation.tool.to_executable_name());

        Self::new(Inner::External(ExternalInvoker::from_command(
            partial_command,
        )))
    }

    fn new(inner: Inner) -> Self {
        Self {
            inner,
            used_flake: None,
        }
    }

    pub fn set_flake(&mut self, flake: NixFlake) {
        self.used_flake = Some(flake);
    }

    pub fn dispatch(self, args: &[OsString]) {
        match self.inner {
            Inner::Internal => todo!(),
            Inner::External(mut v) => {
                v.configure_command(|cmd| {
                    cmd.args(args);
                    if let Some(flake) = &self.used_flake {
                        cmd.env("NIX_RUST_WRANGLER_FLAKE_PATH", flake.path());
                    }
                });

                tracing::trace!("Executing external invoker: {:?}", v);
                let err = v.exec();
                tracing::error!("Failed to execute command: {}", err);
                std::process::exit(1);
            }
        }
    }
}
