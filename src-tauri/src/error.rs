//! The single error type every command returns to the frontend.

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid theme: {0}")]
    InvalidTheme(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("terminal error: {0}")]
    Pty(String),
    /// The file changed on disk since it was read; nothing was written.
    #[error("{0}")]
    Conflict(String),
    #[error("file is too large to open ({size} bytes, limit {limit} bytes)")]
    TooLarge { size: u64, limit: u64 },
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Stable machine-readable kind, mirrored by `AppErrorKind` in `src/shared/types.ts`.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Json(_) => "json",
            Self::InvalidTheme(_) => "invalidTheme",
            Self::NotFound(_) => "notFound",
            Self::InvalidInput(_) => "invalidInput",
            Self::Pty(_) => "pty",
            Self::Conflict(_) => "conflict",
            Self::TooLarge { .. } => "tooLarge",
            Self::Internal(_) => "internal",
        }
    }
}

/// Serialized as `{ "kind": "...", "message": "..." }` so the frontend can show a toast.
/// `TooLarge` also carries `size` (bytes) for the editor's info view.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let size = match self {
            Self::TooLarge { size, .. } => Some(*size),
            _ => None,
        };
        let mut state = serializer.serialize_struct("AppError", 2 + usize::from(size.is_some()))?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        if let Some(size) = size {
            state.serialize_field("size", &size)?;
        }
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_kind_and_message() {
        let err = AppError::InvalidTheme("missing \"colors\" object".into());
        let json = serde_json::to_value(&err).expect("serialize");
        assert_eq!(json["kind"], "invalidTheme");
        assert_eq!(json["message"], "invalid theme: missing \"colors\" object");
    }

    #[test]
    fn too_large_carries_the_size() {
        let err = AppError::TooLarge { size: 7, limit: 5 };
        let json = serde_json::to_value(&err).expect("serialize");
        assert_eq!(json["kind"], "tooLarge");
        assert_eq!(json["size"], 7);
    }

    #[test]
    fn io_errors_convert() {
        let err: AppError = std::io::Error::new(std::io::ErrorKind::NotFound, "gone").into();
        assert_eq!(err.kind(), "io");
    }
}
