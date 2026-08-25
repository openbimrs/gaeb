//! Pure-Rust, lossless tools for GAEB DA XML.
//!
//! `openbim-gaeb` recognizes GAEB by content, cross-checks namespace/header/DP
//! evidence, extracts common bill-of-quantities item views, and preserves the
//! complete source document. Unknown XML remains byte-identical. Supported edits
//! splice only the requested field and then reparse atomically.
//!
//! # Boundary
//!
//! XML mechanics use `quick-xml` directly. BOM handling, content detection,
//! streaming element interpretation, GAEB exchange phases,
//! BoQ semantics, diagnostics, and editing remain here because they are GAEB
//! policy rather than generic XML behavior.
//!
//! # Example
//!
//! ```
//! use openbim_gaeb::{Document, ExchangePhase, GaebVersion};
//!
//! let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3">
//!   <GAEBInfo><Version>3.3</Version><VersDate>2023-01</VersDate></GAEBInfo>
//!   <Award><DP>83</DP></Award>
//! </GAEB>"#;
//! let document = Document::parse(xml)?;
//! assert_eq!(document.metadata().version, Some(GaebVersion::V3_3));
//! assert_eq!(document.metadata().phase, Some(ExchangePhase::X83));
//! assert_eq!(document.as_bytes(), xml);
//! # Ok::<(), openbim_gaeb::Error>(())
//! ```
//!
//! # Capability limits
//!
//! This is not an XSD validator and does not claim complete typed bindings for
//! every phase. [`Item`] is a common read model; unsupported GAEB content remains
//! accessible and preserved through [`Document::as_bytes`].

#![forbid(unsafe_code)]

mod diagnostic;
mod document;
mod error;
mod metadata;
mod model;
mod parser;
mod phase;
mod version;

pub use diagnostic::{Diagnostic, DiagnosticKind};
pub use document::Document;
pub use error::Error;
pub use metadata::Metadata;
pub use model::{CategoryRef, Item};
pub use phase::ExchangePhase;
pub use version::GaebVersion;
