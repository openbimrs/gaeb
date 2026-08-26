use super::{decimal::Decimal, severity_for, tree::Tree};
use crate::{Document, ValidationDiagnostic, ValidationReport, ValidationSeverity};
use roxmltree::NodeId;
use std::collections::HashMap;

pub(super) fn validate(baseline: &Document, candidate: &Document) -> ValidationReport {
    let mut report = ValidationReport::default();
    if !same_release(baseline, candidate) {
        return report;
    }
    let before = match Tree::parse(baseline.as_bytes()) {
        Ok(tree) => tree,
        Err(error) => {
            report.push(ValidationDiagnostic::business(
                "GAEB-BR-ENGINE-001",
                ValidationSeverity::Error,
                format!("baseline business-rule XML view could not be built: {error}"),
            ));
            return report;
        }
    };
    let after = match Tree::parse(candidate.as_bytes()) {
        Ok(tree) => tree,
        Err(error) => {
            report.push(ValidationDiagnostic::business(
                "GAEB-BR-ENGINE-001",
                ValidationSeverity::Error,
                format!("candidate business-rule XML view could not be built: {error}"),
            ));
            return report;
        }
    };
    validate_boq_structure(baseline, candidate, &before, &after, &mut report);
    validate_x83_x84(baseline, candidate, &before, &after, &mut report);
    validate_quantity_links(baseline, candidate, &before, &after, &mut report);
    validate_trade_order_numbers(baseline, candidate, &before, &after, &mut report);
    report
}

fn same_release(baseline: &Document, candidate: &Document) -> bool {
    let before = baseline.metadata();
    let after = candidate.metadata();
    before.version_text.is_some()
        && before.version_text == after.version_text
        && before.version_date.is_some()
        && before.version_date == after.version_date
        && before.namespace.rsplit('/').next() == after.namespace.rsplit('/').next()
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

fn validate_boq_structure(
    baseline: &Document,
    candidate: &Document,
    before: &Tree<'_>,
    after: &Tree<'_>,
    report: &mut ValidationReport,
) {
    if baseline.metadata().phase_code.as_deref() != Some("83")
        || candidate.metadata().phase_code.as_deref() != Some("84")
    {
        return;
    }
    let left = outline_signature(before);
    let right = outline_signature(after);
    if left != right {
        emit(
            report,
            after,
            "GAEB-LINT-BOQ-003",
            after.root(),
            "BoQ breakdown or outline structure changed across the exchange pair",
        );
    }
}

fn outline_signature(tree: &Tree<'_>) -> Vec<(String, String)> {
    let mut signature = tree
        .all_named("BoQBkdn")
        .into_iter()
        .map(|node| {
            (
                "BoQBkdn".to_owned(),
                format!(
                    "{}\u{1f}{}",
                    tree.attribute_signature(node),
                    tree.text(node)
                ),
            )
        })
        .collect::<Vec<_>>();
    for name in ["BoQCtgy", "Item"] {
        for node in tree.all_named(name) {
            signature.push((
                name.to_owned(),
                tree.attribute(node, "RNoPart").unwrap_or("").to_owned(),
            ));
        }
    }
    signature
}

fn validate_x83_x84(
    baseline: &Document,
    candidate: &Document,
    before: &Tree<'_>,
    after: &Tree<'_>,
    report: &mut ValidationReport,
) {
    if baseline.metadata().phase_code.as_deref() != Some("83")
        || candidate.metadata().phase_code.as_deref() != Some("84")
    {
        return;
    }
    let left = item_map(before);
    let right = item_map(after);
    for (id, (left_node, protected, description)) in left {
        let Some((right_node, current, current_description)) = right.get(&id) else {
            emit(
                report,
                after,
                "GAEB-LINT-X84-001",
                after.root(),
                format!("protected tender item {id:?} is missing"),
            );
            continue;
        };
        if protected != *current {
            emit(
                report,
                after,
                "GAEB-LINT-X84-001",
                *right_node,
                format!("protected outline/text/quantity content changed for item {id:?}"),
            );
        }
        if description != *current_description {
            emit(
                report,
                after,
                "GAEB-LINT-DESCR-001",
                *right_node,
                format!("description identity/content changed for item {id:?}"),
            );
        }
        let _ = left_node;
    }

    let left_vat = first_text(before, &["VAT", "VATRate", "VATPercent"]);
    let right_vat = first_text(after, &["VAT", "VATRate", "VATPercent"]);
    if left_vat.is_some() && left_vat != right_vat {
        emit(
            report,
            after,
            "GAEB-LINT-X84-002",
            after.root(),
            "project VAT changed between X83 and X84",
        );
    }

    for complement in after.all_named("TextComplement") {
        if after.attribute(complement, "MarkLbl").is_none() {
            emit(
                report,
                after,
                "GAEB-LINT-TEXT-001",
                complement,
                "text complement was changed or supplied without a designated completion slot",
            );
        }
    }
}

fn item_map(tree: &Tree<'_>) -> HashMap<String, (NodeId, String, String)> {
    tree.all_named("Item")
        .into_iter()
        .filter_map(|node| {
            let id = tree.attribute(node, "ID")?.to_owned();
            let protected = ["RNoPart", "Qty", "QU"]
                .into_iter()
                .map(|name| {
                    tree.attribute(node, name)
                        .map(str::to_owned)
                        .or_else(|| tree.child_text(node, name))
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join("\u{1f}");
            let description = ["Description", "OutlineText", "CompleteText", "TxtOutlTxt"]
                .into_iter()
                .filter_map(|name| {
                    tree.first_child(node, name).map(|description| {
                        format!(
                            "{}\u{1f}{}",
                            tree.attribute(description, "ID").unwrap_or(""),
                            tree.text(description)
                        )
                    })
                })
                .collect::<Vec<_>>()
                .join("\u{1f}");
            Some((id, (node, protected, description)))
        })
        .collect()
}

fn first_text(tree: &Tree<'_>, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        tree.all_named(name)
            .into_iter()
            .next()
            .map(|node| tree.text(node))
    })
}

fn validate_quantity_links(
    baseline: &Document,
    candidate: &Document,
    before: &Tree<'_>,
    after: &Tree<'_>,
    report: &mut ValidationReport,
) {
    let (x31, lv) = if baseline.metadata().phase_code.as_deref() == Some("31") {
        (before, after)
    } else if candidate.metadata().phase_code.as_deref() == Some("31") {
        (after, before)
    } else {
        return;
    };
    let lv_quantities: HashMap<String, Decimal> = lv
        .all_named("Item")
        .into_iter()
        .filter_map(|node| {
            let quantity = lv.child_text(node, "Qty")?;
            Some((
                lv.attribute(node, "ID")?.to_owned(),
                Decimal::parse(&quantity)?,
            ))
        })
        .collect();
    for calculation in x31.all_named("QtyDeterm") {
        let reference = x31
            .attribute(calculation, "IDRef")
            .map(str::to_owned)
            .or_else(|| x31.child_text(calculation, "IDRef"))
            .or_else(|| x31.child_text(calculation, "ItemIDRef"));
        let result = ["Result", "Qty", "TotalQty"].into_iter().find_map(|name| {
            x31.child_text(calculation, name)
                .and_then(|value| Decimal::parse(&value))
        });
        match (reference, result) {
            (Some(reference), Some(result))
                if lv_quantities
                    .get(&reference)
                    .is_some_and(|qty| qty.equals_at(result, qty.scale().max(result.scale()))) => {}
            (Some(reference), Some(_)) if lv_quantities.contains_key(&reference) => emit(
                report,
                x31,
                "GAEB-LINT-QTY-001",
                calculation,
                format!("X31 result differs from linked LV quantity {reference:?}"),
            ),
            _ => emit(
                report,
                x31,
                "GAEB-LINT-QTY-001",
                calculation,
                "X31 quantity calculation has no resolvable LV allocation/result",
            ),
        }
    }
}

fn validate_trade_order_numbers(
    baseline: &Document,
    candidate: &Document,
    before: &Tree<'_>,
    after: &Tree<'_>,
    report: &mut ValidationReport,
) {
    if baseline.metadata().phase_code.as_deref() != Some("96")
        || candidate.metadata().phase_code.as_deref() != Some("97")
    {
        return;
    }
    let expected: HashMap<String, String> = before
        .all_named("OrderItem")
        .into_iter()
        .filter_map(|node| {
            Some((
                before.attribute(node, "ID")?.to_owned(),
                before.attribute(node, "RNoPart")?.to_owned(),
            ))
        })
        .collect();
    for item in after.all_named("OrderItem") {
        let Some(id) = after.attribute(item, "ID") else {
            continue;
        };
        if let Some(expected_number) = expected.get(id) {
            if after.attribute(item, "RNoPart") != Some(expected_number.as_str()) {
                emit(
                    report,
                    after,
                    "GAEB-LINT-TRADE-001",
                    item,
                    format!(
                        "supplier order position number differs from customer order for {id:?}"
                    ),
                );
            }
        }
    }
}
