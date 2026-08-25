use thiserror::Error;

/// Errors that prevent a document or edit from being represented safely.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("input does not look like XML")]
    NotXml,
    #[error("XML root is not a namespaced GAEB document")]
    NotGaeb,
    #[error("invalid XML: {0}")]
    Xml(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0:?} is not an XML Schema decimal lexical value")]
    InvalidDecimal(String),
    #[error("GAEB item {0:?} was not found")]
    ItemNotFound(String),
    #[error("GAEB item ID {0:?} occurs more than once")]
    AmbiguousItem(String),
    #[error("GAEB item {0:?} has no Qty value")]
    QuantityMissing(String),
    #[error(
        "GAEB item {0:?} has a Qty value that cannot be edited without changing non-value XML"
    )]
    QuantityNotEditable(String),
}
