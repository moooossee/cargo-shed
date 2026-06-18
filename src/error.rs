use camino::Utf8PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShedError {
    #[error("could not read {path}: {source}")]
    Read {
        path: Utf8PathBuf,
        source: std::io::Error,
    },
    #[error("could not write {path}: {source}")]
    Write {
        path: Utf8PathBuf,
        source: std::io::Error,
    },
    #[error("could not parse {path}: {message}")]
    Parse { path: Utf8PathBuf, message: String },
    #[error("manifest not found at {path}")]
    ManifestNotFound { path: Utf8PathBuf },
    #[error("path is not valid UTF-8: {path}")]
    NonUtf8Path { path: std::path::PathBuf },
    #[error("unknown rule: {rule_id}")]
    UnknownRule { rule_id: String },
}

pub(crate) fn utf8_path(path: std::path::PathBuf) -> Result<Utf8PathBuf, ShedError> {
    Utf8PathBuf::from_path_buf(path.clone()).map_err(|_| ShedError::NonUtf8Path { path })
}
