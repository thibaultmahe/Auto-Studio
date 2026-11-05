//! Métier local, sans dépendance à Tauri ni interface graphique.
pub mod compose;
pub mod error;
pub mod imaging;
pub mod library;
pub mod model;
pub mod provider;
pub mod storage;

pub use error::{Error, Result};
pub use library::Library;
pub use model::*;
