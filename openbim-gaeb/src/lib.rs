//! Pure-Rust, lossless tools for GAEB DA XML.
//!
//! `openbim-gaeb` recognizes GAEB by content, cross-checks namespace/header/DP
//! evidence, extracts common bill-of-quantities item views, and preserves the
//! complete source document. Unknown XML remains byte-identical. Supported edits
//! splice only the requested field and then reparse atomically.
//!
//! # Boundary
//!
//! This crate uses `quick-xml` directly. BOM/content detection, strict
//! XML/namespace checks, streaming interpretation, GAEB exchange phases, BoQ
//! semantics, diagnostics, and editing live here so the standalone package has
//! no project-owned generic codec dependency.
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

mod business;
mod diagnostic;
mod document;
mod error;
mod metadata;
mod model;
mod parser;
mod phase;
pub mod support;
mod validation;
mod version;
mod xsd;

pub use business::{BusinessRule, BusinessValidator, BUSINESS_RULES};
pub use diagnostic::{Diagnostic, DiagnosticKind};
pub use document::Document;
pub use error::Error;
pub use metadata::Metadata;
pub use model::{CategoryRef, Item};
pub use phase::ExchangePhase;
pub use validation::{ValidationDiagnostic, ValidationLayer, ValidationReport, ValidationSeverity};
pub use version::GaebVersion;
pub use xsd::{GaebSchemaSet, XsdLoadOptions, XsdSchema, XsdSchemaError};
