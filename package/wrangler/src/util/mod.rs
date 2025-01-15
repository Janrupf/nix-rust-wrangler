use std::ffi::{OsStr, OsString};

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
