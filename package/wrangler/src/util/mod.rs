use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

pub fn prepend_paths<I, S>(current: Option<OsString>, new: I) -> OsString
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    match current {
        None => std::env::join_paths(new).unwrap(),
        Some(v) => {
            let new_paths = new.into_iter().map(|s| s.as_ref().to_os_string());
            let split = std::env::split_paths(&v).map(|v| v.into_os_string());

            std::env::join_paths(new_paths.chain(split)).unwrap()
        }
    }
}

pub fn u32_from_env(name: &str) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

pub fn was_dispatched_into_flake() -> bool {
    std::env::var_os("NIX_RUST_WRANGLER_INSIDE_NIX_DEVELOP")
        .map(|v| v.len() > 0)
        .unwrap_or(false)
}

pub fn find_executable_in_path(name: impl AsRef<OsStr>) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|p| p.join(name.as_ref()))
        .find(|p| p.is_file())
}
