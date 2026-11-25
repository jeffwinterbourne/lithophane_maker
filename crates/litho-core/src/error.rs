//! Domain errors returned by the lithophane core engine.

use thiserror::Error;

/// Unified error type across image processing, geometry construction,
/// manifold boolean operations, settings validation, and file export.
#[derive(Debug, Error)]
pub enum LithoError {
    /// Image loading, decoding, or dimension error.
    #[error("image error: {0}")]
    Image(String),

    /// Invalid configuration or combination of parameters.
    #[error("invalid settings: {0}")]
    Settings(String),

    /// Geometric construction error (for example, impossible radius or degenerate profile).
    #[error("geometry error: {0}")]
    Geometry(String),

    /// A constructed or boolean-merged mesh failed manifold / positive-volume verification.
    #[error("non-manifold mesh: {0}")]
    NonManifold(String),

    /// Filesystem I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// STL or 3MF serialization failure.
    #[error("export error: {0}")]
    Export(String),
}
