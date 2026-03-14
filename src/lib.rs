pub mod commands;
pub mod error;
pub mod saga;
pub mod session;
pub mod step;

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaConfig {
    pub name: String,
    pub status: SagaStatus,
    pub current_step: u32,
    pub created_at: String,
    pub plan_file: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SagaStatus {
    Active,
    Completed,
}

impl fmt::Display for SagaStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SagaStatus::Active => write!(f, "active"),
            SagaStatus::Completed => write!(f, "completed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepConfig {
    pub number: u32,
    pub slug: String,
    pub status: StepStatus,
    pub description: String,
    pub context_files: Vec<String>,
    pub created_at: String,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub transcript_file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Pending,
    #[serde(rename = "in-progress")]
    InProgress,
    Completed,
    Blocked,
}

impl fmt::Display for StepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StepStatus::Pending => write!(f, "pending"),
            StepStatus::InProgress => write!(f, "in-progress"),
            StepStatus::Completed => write!(f, "completed"),
            StepStatus::Blocked => write!(f, "blocked"),
        }
    }
}

/// Read input that could be literal text, a file path, or "-" for stdin.
pub fn read_input(value: &str) -> error::Result<String> {
    if value == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else if std::path::Path::new(value).is_file() {
        Ok(std::fs::read_to_string(value)?)
    } else {
        Ok(value.to_string())
    }
}

/// Get current timestamp as YYYYMMDDTHHMMSS string.
pub fn timestamp() -> String {
    chrono::Local::now().format("%Y%m%dT%H%M%S").to_string()
}

/// Get current timestamp as ISO 8601 string.
pub fn timestamp_iso() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}
