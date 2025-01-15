use crate::error::{CollectionError, Error};
use serde::Deserialize;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolchainCollectionMeta {
    pub host_platform: String,
}

#[derive(Debug)]
pub struct ToolchainCollection {
    collection_dir: PathBuf,
    meta: ToolchainCollectionMeta,
}

impl ToolchainCollection {
    pub fn find() -> Option<Self> {
        let dir = std::env::var_os("NIX_RUST_WRANGLER_TOOLCHAIN_COLLECTION")?;
        Self::from_directory(Path::new(&dir))
            .map_err(|e| {
                tracing::error!(
                    "Failed to load toolchain collection from {}: {}",
                    Path::new(&dir).display(),
                    e
                );
            })
            .ok()
    }

    pub fn from_directory(collection_dir: impl Into<PathBuf>) -> Result<Self, CollectionError> {
        let collection_dir = collection_dir.into();
        let meta_path = collection_dir.join("collection.json");
        let meta = serde_json::from_reader(std::fs::File::open(meta_path)?)?;
        Ok(Self {
            collection_dir,
            meta,
        })
    }

    /// Find a tool in the collection.
    pub fn find_tool(
        &self,
        tool_name: &OsStr,
        toolchain_name: Option<impl AsRef<str>>,
        allow_fallback: bool,
    ) -> Result<(PathBuf, PathBuf), Error> {
        let toolchain_name = toolchain_name.as_ref().map(|s| s.as_ref());

        let toolchain_dir = match toolchain_name {
            Some(toolchain_name) => self.toolchain_dir(toolchain_name.as_ref()),
            None => self.default_toolchain_dir(),
        }?;

        let tool_exe = toolchain_dir.join("bin").join(tool_name);
        if tool_exe.is_file() {
            return Ok((toolchain_dir, tool_exe));
        } else if allow_fallback && toolchain_name.is_some() {
            let default_toolchain_dir = match self.default_toolchain_dir() {
                Ok(v) => v,
                Err(CollectionError::ToolchainNotFound(_)) => {
                    return Err(Error::ToolchainDoesNotProvideTool {
                        path: toolchain_dir,
                        tool: tool_name.to_string_lossy().to_string(),
                    })
                }
                Err(e) => return Err(e.into()),
            };

            let tool_exe = default_toolchain_dir.join("bin").join(tool_name);
            if tool_exe.is_file() {
                return Ok((default_toolchain_dir, tool_exe));
            }
        }

        Err(Error::ToolchainDoesNotProvideTool {
            path: toolchain_dir,
            tool: tool_name.to_string_lossy().to_string(),
        })
    }

    pub fn default_toolchain_dir(&self) -> Result<PathBuf, CollectionError> {
        let names = vec!["default", "stable", "beta", "nightly"];

        for name in names {
            match self.toolchain_dir(name) {
                Ok(toolchain_dir) => return Ok(toolchain_dir),
                Err(CollectionError::ToolchainNotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }

        Err(CollectionError::ToolchainNotFound(
            "default, stable, beta, or nightly".to_string(),
        ))
    }

    pub fn toolchain_dir(&self, toolchain_name: &str) -> Result<PathBuf, CollectionError> {
        match self.toolchain_dir_raw(toolchain_name) {
            Ok(toolchain_dir) => Ok(toolchain_dir),
            Err(CollectionError::ToolchainNotFound(_)) => {
                // Retry with the host platform appended, this is how rustup does it
                self.toolchain_dir_raw(&format!("{}-{}", toolchain_name, self.meta.host_platform))
            }
            Err(e) => Err(e),
        }
    }

    fn toolchain_dir_raw(&self, raw_name: &str) -> Result<PathBuf, CollectionError> {
        let toolchain_dir = self.collection_dir.join(raw_name);
        tracing::trace!("Checking for toolchain at {}", toolchain_dir.display());
        
        if !toolchain_dir.exists() {
            return Err(CollectionError::ToolchainNotFound(raw_name.to_string()));
        }

        let toolchain_dir = toolchain_dir.canonicalize()?;
        if !toolchain_dir.is_dir() {
            return Err(CollectionError::ToolchainNotFound(raw_name.to_string()));
        }

        Ok(toolchain_dir)
    }
}
