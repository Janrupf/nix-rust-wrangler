mod collection;
pub mod error;
mod invocation;
mod invoker;
mod nix;
mod util;

use crate::collection::ToolchainCollection;
use crate::invocation::{Invocation, InvokedTool};
use crate::invoker::ToolInvoker;
use crate::nix::config::FlakeInspection;
use crate::nix::flake::NixFlake;
use crate::nix::proxy::run_develop_proxy;
use crate::nix::NixCommand;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn main() {
    let invocation = match Invocation::derive() {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("{}", err);
            std::process::exit(1);
        }
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_env_var("NIX_RUST_WRANGLER_LOG")
                .with_default_directive(tracing::Level::ERROR.into())
                .from_env_lossy(),
        )
        .init();

    tracing::trace!("nix-rust-wrangler version {}", env!("CARGO_PKG_VERSION"));

    if util::u32_from_env("RUST_RECURSION_COUNT") > 20 {
        tracing::error!("RUST_RECURSION_COUNT exceeded 20, aborting to prevent infinite recursion");
        std::process::exit(1);
    }

    tracing::debug!("Invocation: {:#?}", invocation);

    match &invocation.tool {
        InvokedTool::NixDevelopProxy => {
            run_develop_proxy(invocation);
        }
        _ => { /* fall through */ }
    }

    if let InvokedTool::Other(name) = &invocation.tool {
        tracing::warn!("Unknown tool invocation: {}", name.to_string_lossy());
    }

    if let Some((nix_command, flake)) =
        find_nix().and_then(|cmd| NixFlake::find_automatically().map(|f| (cmd, f)))
    {
        tracing::info!("Using flake at {}", flake.path().display());

        let evaluation = match flake.apply_expr_json::<FlakeInspection>(
            &nix_command,
            ".",
            FlakeInspection::APPLY_EXPR,
        ) {
            Ok(v) => v,
            Err(err) => {
                tracing::error!("Failed to evaluate flake: {}", err);
                std::process::exit(1);
            }
        };

        tracing::trace!("Flake evaluation: {:#?}", evaluation);

        let invoker = match evaluation.make_invoker(&nix_command, &flake, &invocation) {
            None => None,
            Some(Ok(v)) => Some(v),
            Some(Err(err)) => {
                tracing::error!("Failed to create tool invoker: {}", err);
                std::process::exit(1);
            }
        };

        if let Some(mut invoker) = invoker {
            invoker.set_flake(flake.clone());
            dispatch(invoker, &invocation);
        }
    }

    let toolchain_collection = match ToolchainCollection::find() {
        None => {
            tracing::error!("No toolchain found in flake and no toolchain collection found");
            std::process::exit(1);
        }
        Some(v) => v,
    };

    let (toolchain_dir, tool_exe) = match toolchain_collection.find_tool(
        &invocation.tool.to_executable_name(),
        invocation.toolchain_override.as_override_name(),
        invocation.tool == InvokedTool::Cargo,
    ) {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("{}", err);
            std::process::exit(1);
        }
    };

    let invoker = ToolInvoker::from_tool_and_toolchain_dir(&tool_exe, &toolchain_dir);
    dispatch(invoker, &invocation);
}

fn find_nix() -> Option<NixCommand> {
    if std::env::var_os("NIX_RUST_WRANGLER_INSIDE_FLAKE")
        .map(|v| v.len() > 0)
        .unwrap_or(false)
    {
        tracing::debug!("Already dispatched into flake, skipping nix command search to prevent infinite recursion");
        return None;
    }

    let nix_command = if std::env::var_os("NIX_RUST_WRANGLER_DISABLE_NIX")
        .map(|v| v.len() > 0)
        .unwrap_or(false)
    {
        tracing::info!("Nix command disabled by NIX_RUST_WRANGLER_DISABLE_NIX");
        None
    } else {
        NixCommand::find()
    };

    match nix_command.as_ref() {
        None => tracing::info!("No nix command found, flake support will be disabled"),
        Some(v) if v.is_usable() && v.flakes_enabled() => {
            tracing::info!("Nix is available with flakes enabled");
        }
        Some(_) => tracing::info!("Nix found, but it is not enabled or flakes are not enabled"),
    }

    nix_command
}

fn dispatch(invoker: ToolInvoker, invocation: &Invocation) {
    tracing::debug!("Dispatching...");
    invoker.dispatch(&invocation.remaining_args);
}
