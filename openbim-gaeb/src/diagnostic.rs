/// A non-fatal inconsistency or unsupported declaration discovered while reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub message: String,
}

impl Diagnostic {
    pub(crate) fn new(kind: DiagnosticKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// Stable categories callers can use without parsing diagnostic prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagnosticKind {
    VersionMismatch,
    PhaseMismatch,
    UnsupportedVersion,
    UnknownPhase,
    DuplicateVersionDeclaration,
    DuplicatePhaseDeclaration,
    MissingItemId,
}
