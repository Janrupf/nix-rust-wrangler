use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("missing argv0")]
    MissingArgv0,

    #[error("no tool specified which should be invoked")]
    MissingTool,

    #[error("the tool specified is not a valid tool name")]
    InvalidToolName,

    #[error("the toolchain specified via the environment is not valid Unicode")]
    ToolchainEnvNameNotUnicode,
    
    #[error("the selected toolchain at '{}' does not provide the tool {tool}", path.display())]
    ToolchainDoesNotProvideTool {
        path: PathBuf,
        tool: String,
    },
    
    #[error("interacting with the flake failed: {0}")]
    Flake(#[from] FlakeEvalError),
    
    #[error("getting toolchain from collection failed: {0}")]
    Collection(#[from] CollectionError),
    
    #[error("unable to determine own executable: {0}")]
    UnableToDetermineOwnExecutable(std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum FlakeEvalError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("nix command failed: {status:?}\nstdout: {stdout}\nstderr: {stderr}")]
    EvalFailed {
        status: std::process::ExitStatus,
        stdout: String,
        stderr: String,
    },

    #[error(transparent)]
    Parse(#[from] serde_json::Error),
    
    #[error("build did not result in a usable toolchain derivation")]
    MissingToolchainDerivation,
}

#[derive(Debug, thiserror::Error)]
pub enum CollectionError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Parse(#[from] serde_json::Error),
    
    #[error("toolchain {0} is not installed in the collection")]
    ToolchainNotFound(String),
}
