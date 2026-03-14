use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No saga found at {path}/.avoid-compaction/")]
    SagaNotFound { path: PathBuf },

    #[error("Saga already exists at {path}/.avoid-compaction/")]
    SagaAlreadyExists { path: PathBuf },

    #[error("Invalid step transition: {from} -> {to}")]
    InvalidStepTransition { from: String, to: String },

    #[error("No current step found")]
    NoCurrentStep,

    #[error("Saga is already complete")]
    SagaComplete,

    #[error("No steps defined yet")]
    NoSteps,

    #[error("Multiple flags cannot read from stdin")]
    MultipleStdin,

    #[error("{0}")]
    Other(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
