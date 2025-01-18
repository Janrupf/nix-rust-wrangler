use crate::invocation::Invocation;
use crate::invoker::ToolInvoker;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

pub fn run_develop_proxy(invocation: Invocation) -> ! {
    if invocation.remaining_args.is_empty() {
        tracing::error!("No command to run in develop proxy");
        std::process::exit(1);
    }

    let (exe, args) = invocation.remaining_args.split_at(1);
    let exe = &exe[0];

    let mut command = Command::new(exe);

    if let Some(cargo_fallback_toolchain) = std::env::var_os("NIX_RUST_WRANGLER_TOOLCHAIN_FALLBACK")
    {
        ToolInvoker::configure_command_for_toolchain(
            Some(Path::new(&cargo_fallback_toolchain)),
            &mut command,
        );
    }

    command.args(args);

    tracing::trace!("Executing command in develop proxy: {:?}", command);
    let err = command.exec();
    tracing::error!("Failed to execute command: {}", err);
    std::process::exit(1);
}
