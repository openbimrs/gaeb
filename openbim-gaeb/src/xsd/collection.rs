use std::{collections::HashMap, path::Path, rc::Rc};

use super::{XsdLoadOptions, XsdSchema, XsdSchemaError};
use crate::{support, Document, ValidationReport};

/// Exact, fixture-backed GAEB schema dispatcher loaded from caller-provided XSD snapshots.
pub struct GaebSchemaSet {
    schemas: HashMap<(&'static str, &'static str), Rc<XsdSchema>>,
}

impl GaebSchemaSet {
    /// Load every unique schema root in [`support::SUPPORT_MATRIX`].
    ///
    /// The official GAEB graph currently trips `xsd-schema`'s stricter
    /// schema-derivation audit, so this loader disables only that schema-level
    /// audit. Instance constraints remain enabled. Callers remain responsible
    /// for authenticating the unmodified official schema bytes.
    pub fn load_official(root: impl AsRef<Path>) -> Result<Self, XsdSchemaError> {
        let root = root.as_ref();
        let options = XsdLoadOptions {
            validate_schema_derivations: false,
        };
        let mut schemas = HashMap::new();
        for entry in support::SUPPORT_MATRIX {
            let key = (entry.snapshot, entry.schema_root);
            if schemas.contains_key(&key) {
                continue;
            }
            let path = root.join(entry.snapshot).join(entry.schema_root);
            schemas.insert(
                key,
                Rc::new(XsdSchema::from_file_with_options(path, options)?),
            );
        }
        Ok(Self { schemas })
    }

    /// Validate a parsed document against its exact version/phase/namespace row.
    pub fn validate_document(
        &self,
        document: &Document,
    ) -> Result<ValidationReport, XsdSchemaError> {
        let entry = support::candidates_for_document(document)
            .next()
            .ok_or_else(|| XsdSchemaError::UnsupportedProfile {
                version: document.metadata().declared_version,
                phase: document.metadata().declared_phase,
                namespace: document.metadata().namespace.clone(),
            })?;
        self.schemas[&(entry.snapshot, entry.schema_root)].validate(document.as_bytes())
    }
}
