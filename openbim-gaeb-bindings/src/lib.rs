//! Opt-in generated GAEB bindings for fixture-proven profiles.

#![forbid(unsafe_code)]
#![allow(dead_code, unused_mut, unused_variables)]

use openbim_gaeb::{support, Document};
use quick_xml::Writer;
use thiserror::Error;
use xsd_parser_types::quick_xml::{DeserializeSync, SerializeSync, SliceReader};

/// Generated GAEB 3.1 (2007-11) common-root bindings.
#[allow(clippy::all, dead_code, unused_variables)]
pub mod v3_1_2007_11 {
    include!("generated/v3_1_2007_11.rs");
}

/// A typed document selected by an exact fixture-backed support-matrix row.
#[derive(Debug)]
pub enum TypedDocument {
    V3_1_2007_11(Box<v3_1_2007_11::GaebElement>),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("document does not match an exact fixture-backed GAEB support row")]
    UnsupportedProfile,
    #[error("profile has no fixture-proven typed binding")]
    UntypedProfile,
    #[error("GAEB XML must be UTF-8 for generated typed bindings: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("generated binding failed to parse document: {0}")]
    Parse(String),
    #[error("generated binding failed to serialize document: {0}")]
    Serialize(String),
}

impl TypedDocument {
    /// Parse through the generated module declared by the exact support row.
    pub fn parse(document: &Document) -> Result<Self, Error> {
        let entry = support::candidates_for_document(document)
            .next()
            .ok_or(Error::UnsupportedProfile)?;
        let module = entry.typed_module.ok_or(Error::UntypedProfile)?;
        let source = std::str::from_utf8(document.as_bytes())?;
        let mut reader = SliceReader::new(source);
        match module {
            "v3_1_2007_11" => {
                let value = v3_1_2007_11::GaebElement::deserialize(&mut reader)
                    .map_err(|error| Error::Parse(error.to_string()))?;
                Ok(Self::V3_1_2007_11(Box::new(value)))
            }
            _ => Err(Error::UntypedProfile),
        }
    }

    /// Serialize with the root element expected by the selected generated family.
    pub fn to_xml(&self) -> Result<Vec<u8>, Error> {
        let mut output = Vec::new();
        let mut writer = Writer::new(&mut output);
        match self {
            Self::V3_1_2007_11(value) => value
                .serialize("GAEB", &mut writer)
                .map_err(|error| Error::Serialize(error.to_string()))?,
        }
        Ok(output)
    }
}
