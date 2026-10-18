//! Typed access to the recovered, original-game inspection assets.
//!
//! These loaders do not implement game rules. Native coordinates are X/Y ground
//! and Z height. Converted models already use `(native X, native Z, -native Y)`;
//! their hierarchy transforms and vertical UV flip are already baked in.

mod model;
mod store;
mod terrain;
mod types;

pub use model::*;
pub use store::*;
pub use terrain::*;
pub use types::*;

/// Rotate the native Z-up basis into a right-handed Y-up renderer basis.
pub fn native_to_render(position: [f32; 3]) -> [f32; 3] {
    [position[0], position[2], -position[1]]
}

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("invalid JSON in {path}: {source}")]
    Json {
        path: std::path::PathBuf,
        source: serde_json::Error,
    },
    #[error("invalid asset: {0}")]
    Invalid(String),
    #[error("unknown map identifier: {0}")]
    UnknownMap(String),
    #[error("no converted model for original resource: {0}")]
    UnknownModel(String),
}

pub type Result<T> = std::result::Result<T, AssetError>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AssetWarning {
    pub source: String,
    pub message: String,
}
