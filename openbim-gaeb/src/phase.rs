use std::fmt;

/// A standardized GAEB exchange phase (`DP`).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExchangePhase {
    X31,
    X50_1,
    X50_2,
    X51_1,
    X51_2,
    X52,
    X61,
    X80,
    X81,
    X82,
    X83,
    X83Z,
    X84,
    X84P,
    X84Z,
    X85,
    X86,
    X86ZE,
    X86ZR,
    X87,
    X88,
    X89,
    X89B,
    X93,
    X94,
    X96,
    X97,
    X98,
    X99,
}

impl ExchangePhase {
    pub(crate) fn from_code(value: &str) -> Option<Self> {
        Some(match value.trim().to_ascii_uppercase().as_str() {
            "31" => Self::X31,
            "50.1" => Self::X50_1,
            "50.2" => Self::X50_2,
            "51.1" => Self::X51_1,
            "51.2" => Self::X51_2,
            "52" => Self::X52,
            "61" => Self::X61,
            "80" => Self::X80,
            "81" => Self::X81,
            "82" => Self::X82,
            "83" => Self::X83,
            "83Z" => Self::X83Z,
            "84" => Self::X84,
            "84P" => Self::X84P,
            "84Z" => Self::X84Z,
            "85" => Self::X85,
            "86" => Self::X86,
            "86ZE" => Self::X86ZE,
            "86ZR" => Self::X86ZR,
            "87" => Self::X87,
            "88" => Self::X88,
            "89" => Self::X89,
            "89B" => Self::X89B,
            "93" => Self::X93,
            "94" => Self::X94,
            "96" => Self::X96,
            "97" => Self::X97,
            "98" => Self::X98,
            "99" => Self::X99,
            _ => return None,
        })
    }

    /// The exact value used by the GAEB `<DP>` element.
    #[must_use]
    pub const fn as_code(self) -> &'static str {
        match self {
            Self::X31 => "31",
            Self::X50_1 => "50.1",
            Self::X50_2 => "50.2",
            Self::X51_1 => "51.1",
            Self::X51_2 => "51.2",
            Self::X52 => "52",
            Self::X61 => "61",
            Self::X80 => "80",
            Self::X81 => "81",
            Self::X82 => "82",
            Self::X83 => "83",
            Self::X83Z => "83Z",
            Self::X84 => "84",
            Self::X84P => "84P",
            Self::X84Z => "84Z",
            Self::X85 => "85",
            Self::X86 => "86",
            Self::X86ZE => "86ZE",
            Self::X86ZR => "86ZR",
            Self::X87 => "87",
            Self::X88 => "88",
            Self::X89 => "89",
            Self::X89B => "89B",
            Self::X93 => "93",
            Self::X94 => "94",
            Self::X96 => "96",
            Self::X97 => "97",
            Self::X98 => "98",
            Self::X99 => "99",
        }
    }
}

impl fmt::Display for ExchangePhase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_code())
    }
}
