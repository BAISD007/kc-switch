use serde::ser::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum KcError {
    #[error("I/O 错误: {0}")]
    Io(String),
    #[error("解析错误: {0}")]
    Parse(String),
    #[error("未找到: {0}")]
    NotFound(String),
    #[error("参数无效: {0}")]
    Invalid(String),
}

impl Serialize for KcError {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for KcError {
    fn from(e: std::io::Error) -> Self {
        KcError::Io(e.to_string())
    }
}

impl From<toml_edit::TomlError> for KcError {
    fn from(e: toml_edit::TomlError) -> Self {
        KcError::Parse(e.to_string())
    }
}

impl From<serde_json::Error> for KcError {
    fn from(e: serde_json::Error) -> Self {
        KcError::Parse(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, KcError>;
