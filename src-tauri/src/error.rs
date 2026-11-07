//! Error type shared by the image pipeline and the Tauri commands.

use std::fmt;

#[derive(Debug)]
pub struct AppError {
    message: String,
}

impl AppError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::new(error.to_string())
    }
}

impl From<image::ImageError> for AppError {
    fn from(error: image::ImageError) -> Self {
        AppError::new(error.to_string())
    }
}

impl From<String> for AppError {
    fn from(message: String) -> Self {
        AppError::new(message)
    }
}

impl From<&str> for AppError {
    fn from(message: &str) -> Self {
        AppError::new(message)
    }
}

pub type AppResult<T> = Result<T, AppError>;
