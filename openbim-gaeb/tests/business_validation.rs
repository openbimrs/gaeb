use openbim_gaeb::{
    BusinessValidator, Document, ValidationLayer, ValidationSeverity, BUSINESS_RULES,
};

const NS33_84: &str = "http://www.gaeb.de/GAEB_DA_XML/DA84/3.3";
const NS31: &str = "http://www.gaeb.de/GAEB_DA_XML/200407";

fn doc(namespace: &str, version: &str, date: &str, phase: &str, body: &str) -> Document {
    let first_end = body.find('>').expect("phase body needs a root element");
    let (open, rest) = body.split_at(first_end + 1);
    Document::parse(format!(
        r#"<GAEB xmlns="{namespace}"><GAEBInfo><Version>{version}</Version><VersDate>{date}</VersDate></GAEBInfo>{open}<DP>{phase}</DP>{rest}</GAEB>"#,
    ))
    .unwrap()
}

#[test]
fn catalog_distinguishes_evidence_backed_rules_from_interoperability_lints() {
    let ids: Vec<_> = BUSINESS_RULES.iter().map(|rule| rule.id).collect();
    assert_eq!(ids.len(), 18);
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 18);
    assert!(ids.contains(&"GAEB-BR-COST-001"));
    assert!(ids.contains(&"GAEB-BR-X84-31-001"));
    assert!(ids.contains(&"GAEB-BR-X84-31-002"));
    let lints: Vec<_> = BUSINESS_RULES
        .iter()
        .filter(|rule| rule.id.starts_with("GAEB-LINT-"))
        .collect();
    assert_eq!(lints.len(), 15);
    assert!(lints
        .iter()
        .all(|rule| rule.severity == ValidationSeverity::Warning));
    let errors: Vec<_> = BUSINESS_RULES
        .iter()
        .filter(|rule| rule.severity == ValidationSeverity::Error)
        .collect();
    assert_eq!(errors.len(), 3);
}

#[test]
fn price_rule_uses_commercial_rounding_and_reports_item_location() {
    let valid = doc(
        NS33_84,
        "3.3",
        "2023-01",
        "84",
        r#"<Award><BoQ><BoQBody><Itemlist><Item ID="i1"><Qty>126.646</Qty><UP>10.123</UP><IT>1282.04</IT></Item></Itemlist></BoQBody></BoQ></Award>"#,
    );
    assert!(!BusinessValidator::new()
        .validate(&valid)
        .has_code("GAEB-LINT-PRICE-001"));

    let invalid = doc(
        NS33_84,
        "3.3",
        "2023-01",
        "84",
        r#"<Award><BoQ><BoQBody><Itemlist><Item ID="i1"><Qty>126.646</Qty><UP>10.123</UP><IT>1282.05</IT></Item></Itemlist></BoQBody></BoQ></Award>"#,
    );
    let report = BusinessValidator::new().validate(&invalid);
    let diagnostic = report
        .diagnostics()
        .iter()
        .find(|d| d.code() == "GAEB-LINT-PRICE-001")
        .unwrap();
    assert_eq!(diagnostic.layer(), ValidationLayer::Business);
    assert!(diagnostic.location().unwrap().contains("Item[@ID='i1']/IT"));
}

#[test]
fn absurd_component_count_is_bounded_and_reported_once() {
    let document = doc(
        NS33_84,
        "3.3",
        "2023-01",
        "84",
        r#"<Award><BoQ><BoQInfo><NoUPComps>1000000</NoUPComps></BoQInfo><BoQBody><Itemlist><Item ID="i1"><UP>1</UP></Item></Itemlist></BoQBody></BoQ></Award>"#,
    );
    let report = BusinessValidator::new().validate(&document);
    let diagnostics: Vec<_> = report
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code() == "GAEB-LINT-PRICE-002")
        .collect();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity(), ValidationSeverity::Warning);
    assert!(diagnostics[0].message().contains("exceeds the six"));
}

#[test]
fn x84_31_rules_are_exactly_version_date_scoped() {
    let missing = doc(
        NS31,
        "3.1",
        "2009-12",
        "84",
        r#"<Award><MarkupItem ID="m1"/></Award>"#,
    );
    let report = BusinessValidator::new().validate(&missing);
    assert!(
        report.has_code("GAEB-BR-X84-31-001"),
        "{:#?}",
        report.diagnostics()
    );
    assert!(report.has_code("GAEB-BR-X84-31-002"));

    let older = doc(NS31, "3.1", "2007-11", "84", r#"<Award/>"#);
    let report = BusinessValidator::new().validate(&older);
    assert!(!report.has_code("GAEB-BR-X84-31-001"));
    assert!(!report.has_code("GAEB-BR-X84-31-002"));
}

#[test]
fn cost_rule_rejects_references_to_billing_elements() {
    let document = doc(
        "http://www.gaeb.de/GAEB_DA_XML/DA50/3.3",
        "3.3",
        "2023-01",
        "50.1",
        r#"<ElementalCosting><CostElement ID="bill"><BillElement>Yes</BillElement></CostElement><CostElement ID="child"><CostElementRef IDRef="bill"/></CostElement></ElementalCosting>"#,
    );
    let report = BusinessValidator::new().validate(&document);
    assert!(
        report.has_code("GAEB-BR-COST-001"),
        "{:#?}",
        report.diagnostics()
    );
}

#[test]
fn trade_price_characteristic_enforces_dependent_fields() {
    let document = doc(
        "http://www.gaeb.de/GAEB_DA_XML/DA96/3.3",
        "3.3",
        "2023-01",
        "96",
        r#"<Order><OrderItem ID="o1"><PriceChara>3</PriceChara><OfferPrice>12.00</OfferPrice><NetPrice>10.00</NetPrice></OrderItem></Order>"#,
    );
    let report = BusinessValidator::new().validate(&document);
    assert!(report.has_code("GAEB-LINT-TRADE-002"));
}

#[test]
fn x83_to_x84_pair_protects_ids_text_and_quantities() {
    let baseline = doc(
        "http://www.gaeb.de/GAEB_DA_XML/DA83/3.3",
        "3.3",
        "2023-01",
        "83",
        r#"<Award><BoQ><BoQBody><Itemlist><Item ID="i1" RNoPart="10"><Qty>2</Qty><Description ID="d1"><CompleteText>base</CompleteText></Description></Item></Itemlist></BoQBody></BoQ></Award>"#,
    );
    let bid = doc(
        NS33_84,
        "3.3",
        "2023-01",
        "84",
        r#"<Award><BoQ><BoQBody><Itemlist><Item ID="i1" RNoPart="11"><Qty>3</Qty><Description ID="d2"><CompleteText>changed</CompleteText></Description></Item></Itemlist></BoQBody></BoQ></Award>"#,
    );
    let report = BusinessValidator::new().validate_pair(&baseline, &bid);
    assert!(
        report.has_code("GAEB-LINT-DESCR-001"),
        "{:#?}",
        report.diagnostics()
    );
    assert!(report.has_code("GAEB-LINT-X84-001"));
}

#[test]
fn conformance_errors_require_coherent_evidence_backed_releases() {
    let contradictory_x84 = doc(
        NS33_84,
        "3.1",
        "2009-12",
        "84",
        r#"<Award><MarkupItem ID="m1"/></Award>"#,
    );
    let report = BusinessValidator::new().validate(&contradictory_x84);
    assert!(!report.has_code("GAEB-BR-X84-31-001"));
    assert!(!report.has_code("GAEB-BR-X84-31-002"));

    let beta_cost = doc(
        "http://www.gaeb.de/GAEB_DA_XML/DA50/3.4",
        "3.4",
        "2025-01",
        "50.1",
        r#"<ElementalCosting><CostElement ID="bill"><BillElement>Yes</BillElement></CostElement><CostElement><CostElementRef IDRef="bill"/></CostElement></ElementalCosting>"#,
    );
    assert!(!BusinessValidator::new()
        .validate(&beta_cost)
        .has_code("GAEB-BR-COST-001"));

    let contradictory_cost = doc(
        "http://www.gaeb.de/GAEB_DA_XML/DA51/3.3",
        "3.3",
        "2023-01",
        "50.1",
        r#"<ElementalCosting><CostElement ID="bill"><BillElement>Yes</BillElement></CostElement><CostElement><CostElementRef IDRef="bill"/></CostElement></ElementalCosting>"#,
    );
    assert!(!BusinessValidator::new()
        .validate(&contradictory_cost)
        .has_code("GAEB-BR-COST-001"));
}

#[test]
fn unit_price_component_counts_are_scoped_to_each_boq() {
    let document = doc(
        NS33_84,
        "3.3",
        "2023-01",
        "84",
        r#"<Award>
          <BoQ><BoQInfo><NoUPComps>2</NoUPComps></BoQInfo><BoQBody><Itemlist>
            <Item ID="a"><UP>10</UP><UPComp1>3</UPComp1><UPComp2>3</UPComp2></Item>
          </Itemlist></BoQBody></BoQ>
          <BoQ><BoQInfo><NoUPComps>2</NoUPComps></BoQInfo><BoQBody><Itemlist>
            <Item ID="b"><UP>10</UP><UPComp1>10</UPComp1></Item>
          </Itemlist></BoQBody></BoQ>
        </Award>"#,
    );
    let report = BusinessValidator::new().validate(&document);
    assert!(
        report.has_code("GAEB-LINT-PRICE-003"),
        "{:#?}",
        report.diagnostics()
    );
    assert!(
        report.has_code("GAEB-LINT-PRICE-002"),
        "{:#?}",
        report.diagnostics()
    );
}

#[test]
fn item_total_accounts_for_item_discount_percentage() {
    let document = doc(
        NS33_84,
        "3.3",
        "2023-01",
        "84",
        r#"<Award><BoQ><BoQBody><Itemlist><Item ID="i1"><Qty>2</Qty><UP>10</UP><DiscountPcnt>10</DiscountPcnt><IT>18.00</IT></Item></Itemlist></BoQBody></BoQ></Award>"#,
    );
    let report = BusinessValidator::new().validate(&document);
    assert!(
        !report.has_code("GAEB-LINT-PRICE-001"),
        "{:#?}",
        report.diagnostics()
    );
}
