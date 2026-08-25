use crate::{ExchangePhase, GaebVersion};

/// Evidence and generator information extracted from a GAEB document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    /// Root element namespace, preserved verbatim.
    pub namespace: String,
    /// Namespace-first effective version. Inspect diagnostics when evidence differs.
    pub version: Option<GaebVersion>,
    /// Version encoded structurally by the namespace, when available.
    pub namespace_version: Option<GaebVersion>,
    /// Version declared by `<GAEBInfo><Version>`.
    pub declared_version: Option<GaebVersion>,
    /// Uninterpreted `<Version>` text, including future values.
    pub version_text: Option<String>,
    pub version_date: Option<String>,
    /// Namespace-first effective exchange phase.
    pub phase: Option<ExchangePhase>,
    /// Phase encoded structurally by a 3.2+ namespace.
    pub namespace_phase: Option<ExchangePhase>,
    /// Phase declared by `<DP>`.
    pub declared_phase: Option<ExchangePhase>,
    /// Uninterpreted `<DP>` text, including future values.
    pub phase_code: Option<String>,
    pub date: Option<String>,
    pub time: Option<String>,
    pub program_system: Option<String>,
    pub program_name: Option<String>,
}

impl Metadata {
    pub(crate) fn new(namespace: String) -> Self {
        Self {
            namespace,
            version: None,
            namespace_version: None,
            declared_version: None,
            version_text: None,
            version_date: None,
            phase: None,
            namespace_phase: None,
            declared_phase: None,
            phase_code: None,
            date: None,
            time: None,
            program_system: None,
            program_name: None,
        }
    }
}
