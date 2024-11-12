use crate::error::{Error, Result};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use tower_lsp::lsp_types::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RelPathBuf {
    root_dir: PathBuf,
    path_buf: PathBuf,
}

impl std::fmt::Display for RelPathBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.root_dir.display(), self.path_buf.display())
    }
}

impl RelPathBuf {
    pub(crate) fn from_uri(root_dir: PathBuf, uri: Url) -> Result<Self> {
        // Compute the relative path from root_dir to uri assuming uri is a file path.
        let path_buf = uri
            .to_file_path()
            .map_err(|_| "uri is not a file path")?
            .strip_prefix(&root_dir)
            .map_err(|e| Error::from(format!("uri is not a child of root_dir [{e}]")))?
            .to_path_buf();
        Ok(Self { root_dir, path_buf })
    }

    pub(crate) fn from_filename(root_dir: &Path, filename: &str) -> Result<Self> {
        let path_buf = PathBuf::from(filename);
        if path_buf.is_relative() {
            Ok(Self {
                root_dir: root_dir.to_path_buf(),
                path_buf,
            })
        } else {
            let path_buf = path_buf
                .strip_prefix(root_dir)
                .map_err(|e| Error::from(format!("filename prefix could not be stripped [{e}]")))?
                .to_path_buf();
            Ok(Self {
                root_dir: root_dir.to_path_buf(),
                path_buf,
            })
        }
    }
}

impl Deref for RelPathBuf {
    type Target = PathBuf;
    fn deref(&self) -> &Self::Target {
        &self.path_buf
    }
}
