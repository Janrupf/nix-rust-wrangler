use std::ffi::OsStr;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

#[derive(Debug)]
pub(super) struct ExternalInvoker {
    command: Command,
}

impl ExternalInvoker {
    pub fn new_at(directory: &Path, executable: impl AsRef<OsStr>) -> Option<Self> {
        let mut executable = directory.join(Path::new(executable.as_ref()));
        if let Ok(canonical) = executable.canonicalize() {
            executable = canonical;
        }

        if executable.is_file() || executable.is_symlink() {
            Some(Self::from_executable(executable))
        } else {
            None
        }
    }

    pub fn from_executable(executable: impl AsRef<OsStr>) -> Self {
        Self::from_command(Command::new(executable))
    }

    pub fn from_command(partial_command: Command) -> Self {
        Self {
            command: partial_command,
        }
    }
    
    pub fn configure_command(&mut self, configure: impl FnOnce(&mut Command)) {
        configure(&mut self.command);
    }
    
    pub fn exec(mut self) -> std::io::Error {
        self.command.exec()
    }
}
