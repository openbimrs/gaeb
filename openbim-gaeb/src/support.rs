use crate::{Document, ExchangePhase, GaebVersion};

/// One fixture-backed GAEB exchange profile supported by typed and XSD APIs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SupportEntry {
    pub version: GaebVersion,
    pub snapshot: &'static str,
    /// Exact `<GAEBInfo><VersDate>` value proven for this schema profile.
    pub version_date: &'static str,
    pub phase: ExchangePhase,
    /// Profile discriminator when one namespace/phase has multiple schemas.
    pub variant: Option<&'static str>,
    pub namespace: &'static str,
    pub schema_root: &'static str,
    pub fixture: &'static str,
    /// Generated module proven by fixture parse/write/reparse, when available.
    pub typed_module: Option<&'static str>,
}

include!(concat!(env!("OUT_DIR"), "/support_matrix.rs"));

/// Return every exact support row matching declared document metadata.
///
/// Each profile has an exact namespace, so in-memory dispatch does not rely on
/// a file extension or filename.
pub fn candidates_for_document(document: &Document) -> impl Iterator<Item = &'static SupportEntry> {
    let version = document.metadata().declared_version;
    let phase = document.metadata().declared_phase;
    let namespace = document.metadata().namespace.clone();
    let version_date = document.metadata().version_date.clone();
    SUPPORT_MATRIX.iter().filter(move |entry| {
        Some(entry.version) == version
            && Some(entry.phase) == phase
            && entry.namespace == namespace
            && Some(entry.version_date) == version_date.as_deref()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_has_unique_fixture_and_profile_rows() {
        assert_eq!(SUPPORT_MATRIX.len(), 8);
        for (index, left) in SUPPORT_MATRIX.iter().enumerate() {
            assert!(SUPPORT_MATRIX[index + 1..]
                .iter()
                .all(|right| left.fixture != right.fixture));
            assert!(SUPPORT_MATRIX[index + 1..].iter().all(|right| {
                (left.version, left.version_date, left.phase, left.variant)
                    != (
                        right.version,
                        right.version_date,
                        right.phase,
                        right.variant,
                    )
            }));
        }
    }

    #[test]
    fn every_row_is_fixture_backed() {
        let csv = include_str!("../support-matrix.csv");
        for entry in SUPPORT_MATRIX {
            assert!(csv.contains(entry.fixture));
            assert!(csv.contains(entry.schema_root));
            if let Some(module) = entry.typed_module {
                assert!(csv.contains(module));
            }
        }
    }
}
