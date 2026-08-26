mod decimal;
mod pair;
mod single;
mod tree;

use crate::{Document, ValidationReport, ValidationSeverity};

/// Stable metadata for one GAEB business rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BusinessRule {
    pub id: &'static str,
    pub severity: ValidationSeverity,
    pub summary: &'static str,
    pub applicability: &'static str,
}

const fn rule(
    id: &'static str,
    summary: &'static str,
    applicability: &'static str,
) -> BusinessRule {
    BusinessRule {
        id,
        severity: ValidationSeverity::Error,
        summary,
        applicability,
    }
}

const fn lint(
    id: &'static str,
    summary: &'static str,
    applicability: &'static str,
) -> BusinessRule {
    BusinessRule {
        id,
        severity: ValidationSeverity::Warning,
        summary,
        applicability,
    }
}

pub(super) fn severity_for(id: &str) -> ValidationSeverity {
    BUSINESS_RULES
        .iter()
        .find(|rule| rule.id == id)
        .map_or(ValidationSeverity::Error, |rule| rule.severity)
}

/// Evidence-reviewed rule and interoperability-lint catalog.
pub static BUSINESS_RULES: &[BusinessRule] = &[
    lint(
        "GAEB-LINT-BOQ-001",
        "Numeric outline parts when the corresponding breakdown level is numeric",
        "80..89B with BoQBkdn",
    ),
    lint(
        "GAEB-LINT-BOQ-002",
        "Composed outline keys contain at most fourteen characters",
        "80..89B with BoQBkdn",
    ),
    lint(
        "GAEB-LINT-BOQ-003",
        "BoQ breakdown structure is unchanged across an exchange sequence",
        "coherent paired X83 to X84",
    ),
    lint(
        "GAEB-LINT-PRICE-001",
        "Item total equals commercially rounded quantity multiplied by unit price",
        "priced items",
    ),
    lint(
        "GAEB-LINT-PRICE-002",
        "Declared unit-price components are contiguous and complete",
        "NoUPComps != 0",
    ),
    lint(
        "GAEB-LINT-PRICE-003",
        "Unit-price components sum to the unit price",
        "NoUPComps != 0",
    ),
    lint(
        "GAEB-LINT-TOTAL-001",
        "Totals equal included subordinate item and category totals",
        "nodes carrying Totals",
    ),
    lint(
        "GAEB-LINT-TOTAL-002",
        "VAT parts and gross totals are arithmetically consistent",
        "Totals carrying VAT fields",
    ),
    lint(
        "GAEB-LINT-DESCR-001",
        "Description IDs remain unchanged",
        "paired X83 to X84",
    ),
    lint(
        "GAEB-LINT-X84-001",
        "Protected tender outline, text, and quantity content remains unchanged",
        "paired X83 to X84",
    ),
    lint(
        "GAEB-LINT-X84-002",
        "Project VAT remains unchanged",
        "coherent paired X83 to X84",
    ),
    lint(
        "GAEB-LINT-TEXT-001",
        "Only designated text complements are completed",
        "paired X83 to X84",
    ),
    rule(
        "GAEB-BR-X84-31-001",
        "CTR is present",
        "GAEB 3.1 2009-12 X84",
    ),
    rule(
        "GAEB-BR-X84-31-002",
        "Every MarkupItem contains ITMarkup",
        "GAEB 3.1 2009-12 X84",
    ),
    lint(
        "GAEB-LINT-QTY-001",
        "Linked X31 calculation results equal their LV quantities",
        "paired X31 and LV documents",
    ),
    rule(
        "GAEB-BR-COST-001",
        "Billing cost elements are not referenced by another cost element",
        "50.1/50.2/51.1/51.2",
    ),
    lint(
        "GAEB-LINT-TRADE-001",
        "Supplier order position numbers equal customer order position numbers",
        "paired X96 to X97",
    ),
    lint(
        "GAEB-LINT-TRADE-002",
        "PriceChara and dependent price fields form a legal combination",
        "trade price details",
    ),
];

/// Stateless GAEB business-rule validator.
#[derive(Debug, Clone, Copy, Default)]
pub struct BusinessValidator;

impl BusinessValidator {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Validate all single-document rules applicable to `document`.
    #[must_use]
    pub fn validate(&self, document: &Document) -> ValidationReport {
        single::validate(document)
    }

    /// Validate all cross-document rules applicable to an ordered exchange pair.
    #[must_use]
    pub fn validate_pair(&self, baseline: &Document, candidate: &Document) -> ValidationReport {
        pair::validate(baseline, candidate)
    }
}
