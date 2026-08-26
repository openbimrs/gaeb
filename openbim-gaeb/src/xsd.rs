#[path = "xsd/collection.rs"]
mod collection;
#[path = "xsd/single.rs"]
mod single;

pub use collection::GaebSchemaSet;
pub use single::XsdSchema;

/// Controls schema-level checks while a schema graph is loaded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XsdLoadOptions {
    /// Check type-derivation restrictions while loading the schema graph.
    pub validate_schema_derivations: bool,
}

impl Default for XsdLoadOptions {
    fn default() -> Self {
        Self {
            validate_schema_derivations: true,
        }
    }
}

/// Loading or streaming-validation failure.
#[derive(Debug, thiserror::Error)]
pub enum XsdSchemaError {
    #[error("failed to read schema {path}: {source}")]
    Read {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("failed to load schema {path}: {message}")]
    Load {
        path: std::path::PathBuf,
        message: String,
    },
    #[error("failed to validate XML stream: {0}")]
    Validation(String),
    #[error("no fixture-backed schema profile matches version={version:?}, phase={phase:?}, namespace={namespace:?}")]
    UnsupportedProfile {
        version: Option<crate::GaebVersion>,
        phase: Option<crate::ExchangePhase>,
        namespace: String,
    },
}
