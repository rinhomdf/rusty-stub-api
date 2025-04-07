use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Failed to pase OpenAPI spec: {0}")]
    ParseError(String),

    #[error("Failed to read file: {0}")]
    FileError(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    #[error("Internal server error: {0}")]
    InternalSeverError(#[from] actix_web::Error),
}
