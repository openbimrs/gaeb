use std::fmt;

/// A GAEB DA XML schema generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GaebVersion {
    /// Legacy GAEB DA XML 3.1 using the shared `200407` namespace.
    V3_1,
    /// GAEB DA XML 3.2.
    V3_2,
    /// GAEB DA XML 3.3, the current stable generation.
    V3_3,
    /// GAEB DA XML 3.4 as published in the official 2026-03 beta bundle.
    V3_4Beta,
}

impl GaebVersion {
    /// The stable generation new production documents should target.
    pub const CURRENT: Self = Self::V3_3;

    pub(crate) fn from_text(value: &str) -> Option<Self> {
        match value.trim() {
            "3.1" => Some(Self::V3_1),
            "3.2" => Some(Self::V3_2),
            "3.3" => Some(Self::V3_3),
            "3.4" => Some(Self::V3_4Beta),
            _ => None,
        }
    }

    /// The generation string used in GAEB XML.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V3_1 => "3.1",
            Self::V3_2 => "3.2",
            Self::V3_3 => "3.3",
            Self::V3_4Beta => "3.4",
        }
    }

    /// Whether this generation is currently published only as beta material.
    #[must_use]
    pub const fn is_beta(self) -> bool {
        matches!(self, Self::V3_4Beta)
    }
}

impl fmt::Display for GaebVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
