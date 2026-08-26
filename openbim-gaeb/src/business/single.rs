use super::{decimal::Decimal, severity_for, tree::Tree};
use crate::{Document, ValidationDiagnostic, ValidationReport, ValidationSeverity};
use roxmltree::NodeId;
use std::collections::HashSet;

pub(super) fn validate(document: &Document) -> ValidationReport {
    let mut report = ValidationReport::default();
    let tree = match Tree::parse(document.as_bytes()) {
        Ok(tree) => tree,
        Err(error) => {
            report.push(ValidationDiagnostic::business(
                "GAEB-BR-ENGINE-001",
                ValidationSeverity::Error,
                format!("business-rule XML view could not be built: {error}"),
            ));
            return report;
        }
    };
    validate_boq(&tree, &mut report);
    validate_prices(&tree, &mut report);
    validate_totals(&tree, &mut report);
    validate_x84_31(document, &tree, &mut report);
    validate_cost_elements(document, &tree, &mut report);
    validate_trade_prices(&tree, &mut report);
    report
}

fn emit(
    report: &mut ValidationReport,
    tree: &Tree<'_>,
    id: &'static str,
    node: NodeId,
    message: impl Into<String>,
) {
    report.push(
        ValidationDiagnostic::business(id, severity_for(id), message)
            .at_location(tree.location(node)),
    );
}

fn validate_boq(tree: &Tree<'_>, report: &mut ValidationReport) {
    let numeric_levels: Vec<bool> = tree
        .all_named("BoQBkdn")
        .into_iter()
        .filter_map(|node| tree.child_text(node, "Num"))
        .map(|value| value.eq_ignore_ascii_case("yes") || value == "1")
        .collect();
    if numeric_levels.is_empty() {
        return;
    }
    for node in tree
        .all_named("Item")
        .into_iter()
        .chain(tree.all_named("BoQCtgy"))
    {
        let mut parts = Vec::new();
        let mut current = Some(node);
        while let Some(id) = current {
            if let Some(part) = tree.attribute(id, "RNoPart") {
                parts.push(part.to_owned());
            }
            current = tree.parent(id);
        }
        parts.reverse();
        for (index, part) in parts.iter().enumerate() {
            if numeric_levels.get(index).copied().unwrap_or(false)
                && !part.bytes().all(|byte| byte.is_ascii_digit())
            {
                emit(
                    report,
                    tree,
                    "GAEB-LINT-BOQ-001",
                    node,
                    format!(
                        "outline part {part:?} at numeric level {} contains non-digits",
                        index + 1
                    ),
                );
            }
        }
        let composed: String = parts.iter().map(String::as_str).collect();
        if composed.chars().count() > 14 {
            emit(
                report,
                tree,
                "GAEB-LINT-BOQ-002",
                node,
                format!("composed outline key {composed:?} exceeds fourteen characters"),
            );
        }
    }
}

fn validate_prices(tree: &Tree<'_>, report: &mut ValidationReport) {
    const MAX_UNIT_PRICE_COMPONENTS: usize = 6;

    for item in tree.all_named("Item") {
        if let (Some(qty), Some(up), Some(total)) = (
            decimal_child(tree, item, "Qty"),
            decimal_child(tree, item, "UP"),
            decimal_child(tree, item, "IT"),
        ) {
            let expected = decimal_child(tree, item, "DiscountPcnt").map_or_else(
                || qty.multiply_rounded(up, 2),
                |discount| qty.multiply_discounted_rounded(up, discount, 2),
            );
            if !expected.is_some_and(|expected| expected.equals_at(total, 2)) {
                emit(
                    report,
                    tree,
                    "GAEB-LINT-PRICE-001",
                    tree.first_child(item, "IT").unwrap_or(item),
                    "item total differs from commercially rounded Qty × UP after item discount",
                );
            }
        }

        let component_count = nearest_declared_component_count(tree, item);
        if let Some(count) = component_count.filter(|count| *count > 0) {
            if count > MAX_UNIT_PRICE_COMPONENTS {
                emit(
                    report,
                    tree,
                    "GAEB-LINT-PRICE-002",
                    item,
                    format!("NoUPComps {count} exceeds the six UPComp fields defined by GAEB"),
                );
                continue;
            }
            let mut components = Vec::new();
            let mut missing = Vec::new();
            for index in 1..=count {
                let name = format!("UPComp{index}");
                match tree
                    .child_text(item, &name)
                    .and_then(|value| Decimal::parse(&value))
                {
                    Some(value) => components.push(value),
                    None => missing.push(index),
                }
            }
            for index in (count + 1)..=6 {
                let name = format!("UPComp{index}");
                if tree.child_text(item, &name).is_some() {
                    missing.push(index);
                }
            }
            if !missing.is_empty() {
                emit(report, tree, "GAEB-LINT-PRICE-002", item, format!("unit-price component declaration is incomplete or non-contiguous at indices {missing:?}"));
            } else if let (Some(up), Some(sum)) = (
                decimal_child(tree, item, "UP"),
                components
                    .into_iter()
                    .try_fold(Decimal::parse("0").unwrap(), |sum, value| sum.add(value)),
            ) {
                if !sum.equals_at(up, up.scale().max(sum.scale())) {
                    emit(
                        report,
                        tree,
                        "GAEB-LINT-PRICE-003",
                        tree.first_child(item, "UP").unwrap_or(item),
                        "unit-price components do not sum to UP",
                    );
                }
            }
        }
    }
}

fn nearest_declared_component_count(tree: &Tree<'_>, node: NodeId) -> Option<usize> {
    let mut current = Some(node);
    while let Some(id) = current {
        if let Some(value) = tree.child_text(id, "NoUPComps") {
            return value.parse().ok();
        }
        if tree.is(id, "BoQ") {
            if let Some(value) = tree
                .first_child(id, "BoQInfo")
                .and_then(|info| tree.child_text(info, "NoUPComps"))
            {
                return value.parse().ok();
            }
        }
        current = tree.parent(id);
    }
    let declared = tree.all_named("NoUPComps");
    if declared.len() == 1 {
        return tree.text(declared[0]).parse().ok();
    }
    None
}

fn decimal_child(tree: &Tree<'_>, node: NodeId, name: &str) -> Option<Decimal> {
    tree.child_text(node, name)
        .and_then(|value| Decimal::parse(&value))
}

fn validate_totals(tree: &Tree<'_>, report: &mut ValidationReport) {
    for totals in tree.all_named("Totals") {
        let Some(declared) =
            direct_decimal_any(tree, totals, &["Total", "TotalNet", "TotalAmount"])
        else {
            continue;
        };
        let Some(parent) = tree.parent(totals) else {
            continue;
        };
        let mut sum = Decimal::parse("0").unwrap();
        let mut found = false;
        for child in tree.children(parent) {
            if child == totals || tree.element(child).is_none() {
                continue;
            }
            if tree
                .child_text(child, "NotOffered")
                .is_some_and(|value| value.eq_ignore_ascii_case("yes") || value == "1")
            {
                continue;
            }
            if let Some(value) =
                direct_decimal_any(tree, child, &["IT", "Total", "TotalNet", "TotalAmount"])
            {
                if let Some(next) = sum.add(value) {
                    sum = next;
                    found = true;
                }
            }
        }
        if found && !sum.equals_at(declared, 2) {
            emit(
                report,
                tree,
                "GAEB-LINT-TOTAL-001",
                totals,
                "declared total differs from included subordinate totals",
            );
        }

        if let (Some(net), Some(vat), Some(gross)) = (
            direct_decimal_any(tree, totals, &["TotalNet", "NetTotal"]),
            direct_decimal_any(tree, totals, &["VATAmount", "VAT"]),
            direct_decimal_any(tree, totals, &["TotalGross", "GrossTotal"]),
        ) {
            if !net.add(vat).is_some_and(|value| value.equals_at(gross, 2)) {
                emit(
                    report,
                    tree,
                    "GAEB-LINT-TOTAL-002",
                    totals,
                    "gross total differs from net total plus VAT amount",
                );
            }
        }
    }
}

fn direct_decimal_any(tree: &Tree<'_>, node: NodeId, names: &[&str]) -> Option<Decimal> {
    names
        .iter()
        .find_map(|name| decimal_child(tree, node, name))
}

fn validate_x84_31(document: &Document, tree: &Tree<'_>, report: &mut ValidationReport) {
    let metadata = document.metadata();
    if metadata.namespace != "http://www.gaeb.de/GAEB_DA_XML/200407"
        || metadata.version_text.as_deref() != Some("3.1")
        || metadata.version_date.as_deref() != Some("2009-12")
        || metadata.phase_code.as_deref() != Some("84")
    {
        return;
    }
    let award = tree.all_named("Award").into_iter().next();
    match award {
        Some(node) if tree.child_text(node, "CTR").is_none() => emit(
            report,
            tree,
            "GAEB-BR-X84-31-001",
            node,
            "CTR is required for GAEB 3.1 2009-12 X84",
        ),
        None => emit(
            report,
            tree,
            "GAEB-BR-X84-31-001",
            tree.root(),
            "Award/CTR is required for GAEB 3.1 2009-12 X84",
        ),
        _ => {}
    }
    for markup in tree.all_named("MarkupItem") {
        if tree.child_text(markup, "ITMarkup").is_none() {
            emit(
                report,
                tree,
                "GAEB-BR-X84-31-002",
                markup,
                "MarkupItem is missing ITMarkup",
            );
        }
    }
}

fn validate_cost_elements(document: &Document, tree: &Tree<'_>, report: &mut ValidationReport) {
    let metadata = document.metadata();
    let phase_from_namespace = metadata
        .namespace
        .strip_prefix("http://www.gaeb.de/GAEB_DA_XML/DA")
        .and_then(|rest| rest.split_once('/'))
        .map(|(phase, _)| phase);
    let phase_code = metadata.phase_code.as_deref();
    let phase_family = phase_code.and_then(|phase| phase.split('.').next());
    let applicable = metadata.version_text.as_deref() == Some("3.3")
        && metadata.namespace.ends_with("/3.3")
        && phase_from_namespace == phase_family
        && matches!(phase_code, Some("50.1" | "50.2" | "51.1" | "51.2"));
    if !applicable {
        return;
    }
    let billing_ids: HashSet<String> = tree
        .all_named("CostElement")
        .into_iter()
        .filter(|node| {
            tree.child_text(*node, "BillElement")
                .is_some_and(|value| value.eq_ignore_ascii_case("yes") || value == "1")
        })
        .filter_map(|node| tree.attribute(node, "ID").map(str::to_owned))
        .collect();
    for reference in tree.all_named("CostElementRef") {
        let target = tree
            .attribute(reference, "IDRef")
            .map(str::to_owned)
            .or_else(|| tree.child_text(reference, "IDRef"));
        if target.as_ref().is_some_and(|id| billing_ids.contains(id)) {
            emit(
                report,
                tree,
                "GAEB-BR-COST-001",
                reference,
                format!("billing cost element {target:?} is referenced by another cost element"),
            );
        }
    }
}

fn validate_trade_prices(tree: &Tree<'_>, report: &mut ValidationReport) {
    for item in tree.all_named("OrderItem") {
        let Some(character) = tree.child_text(item, "PriceChara") else {
            continue;
        };
        let has_offer = tree.child_text(item, "OfferPrice").is_some();
        let has_net = tree.child_text(item, "NetPrice").is_some();
        let has_modification = tree.first_child(item, "PriceModification").is_some();
        let valid = match character.as_str() {
            "1" => has_offer && has_net && has_modification,
            "2" => has_offer && has_net,
            "3" => !has_offer && !has_modification && has_net,
            _ => false,
        };
        if !valid {
            emit(
                report,
                tree,
                "GAEB-LINT-TRADE-002",
                item,
                format!(
                    "PriceChara {character:?} has an invalid dependent price-field combination"
                ),
            );
        }
    }
}
