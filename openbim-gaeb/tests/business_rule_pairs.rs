use openbim_gaeb::{BusinessValidator, Document, ValidationSeverity};

fn doc(namespace: &str, version: &str, date: &str, phase: &str, body: &str) -> Document {
    let first_end = body.find('>').unwrap();
    let (open, rest) = body.split_at(first_end + 1);
    Document::parse(format!(
        r#"<GAEB xmlns="{namespace}"><GAEBInfo><Version>{version}</Version><VersDate>{date}</VersDate></GAEBInfo>{open}<DP>{phase}</DP>{rest}</GAEB>"#,
    ))
    .unwrap()
}

fn v33(phase: &str, body: &str) -> Document {
    doc(
        &format!("http://www.gaeb.de/GAEB_DA_XML/DA{phase}/3.3"),
        "3.3",
        "2023-01",
        phase,
        body,
    )
}

#[test]
fn boq_rules_have_positive_and_negative_cases() {
    let valid = v33(
        "83",
        r#"<Award><BoQ><BoQInfo><BoQBkdn><Num>Yes</Num></BoQBkdn></BoQInfo><BoQBody><Itemlist><Item ID="i" RNoPart="123"/></Itemlist></BoQBody></BoQ></Award>"#,
    );
    let report = BusinessValidator::new().validate(&valid);
    assert!(!report.has_code("GAEB-LINT-BOQ-001"));
    assert!(!report.has_code("GAEB-LINT-BOQ-002"));

    let invalid = v33(
        "83",
        r#"<Award><BoQ><BoQInfo><BoQBkdn><Num>Yes</Num></BoQBkdn></BoQInfo><BoQBody><Itemlist><Item ID="i" RNoPart="12345678901234A"/></Itemlist></BoQBody></BoQ></Award>"#,
    );
    let report = BusinessValidator::new().validate(&invalid);
    assert!(report.has_code("GAEB-LINT-BOQ-001"));
    assert!(report.has_code("GAEB-LINT-BOQ-002"));
}

#[test]
fn unit_price_component_rules_have_positive_and_negative_cases() {
    let valid = v33(
        "83",
        r#"<Award><BoQ><BoQInfo><NoUPComps>2</NoUPComps></BoQInfo><BoQBody><Itemlist><Item ID="i"><UP>3.00</UP><UPComp1>1.00</UPComp1><UPComp2>2.00</UPComp2></Item></Itemlist></BoQBody></BoQ></Award>"#,
    );
    let report = BusinessValidator::new().validate(&valid);
    assert!(!report.has_code("GAEB-LINT-PRICE-002"));
    assert!(!report.has_code("GAEB-LINT-PRICE-003"));

    let missing = v33(
        "83",
        r#"<Award><BoQ><BoQInfo><NoUPComps>2</NoUPComps></BoQInfo><BoQBody><Itemlist><Item ID="i"><UP>3.00</UP><UPComp1>1.00</UPComp1></Item></Itemlist></BoQBody></BoQ></Award>"#,
    );
    assert!(BusinessValidator::new()
        .validate(&missing)
        .has_code("GAEB-LINT-PRICE-002"));

    let wrong_sum = v33(
        "83",
        r#"<Award><BoQ><BoQInfo><NoUPComps>2</NoUPComps></BoQInfo><BoQBody><Itemlist><Item ID="i"><UP>4.00</UP><UPComp1>1.00</UPComp1><UPComp2>2.00</UPComp2></Item></Itemlist></BoQBody></BoQ></Award>"#,
    );
    assert!(BusinessValidator::new()
        .validate(&wrong_sum)
        .has_code("GAEB-LINT-PRICE-003"));
}

#[test]
fn total_rules_have_positive_and_negative_cases() {
    let valid = v33(
        "86",
        r#"<Award><Container><Item><IT>2.00</IT></Item><Totals><TotalNet>2.00</TotalNet><VATAmount>0.38</VATAmount><TotalGross>2.38</TotalGross></Totals></Container></Award>"#,
    );
    let report = BusinessValidator::new().validate(&valid);
    assert!(!report.has_code("GAEB-LINT-TOTAL-001"));
    assert!(!report.has_code("GAEB-LINT-TOTAL-002"));

    let invalid = v33(
        "86",
        r#"<Award><Container><Item><IT>2.00</IT></Item><Totals><TotalNet>3.00</TotalNet><VATAmount>0.38</VATAmount><TotalGross>4.00</TotalGross></Totals></Container></Award>"#,
    );
    let report = BusinessValidator::new().validate(&invalid);
    assert!(report.has_code("GAEB-LINT-TOTAL-001"));
    assert!(report.has_code("GAEB-LINT-TOTAL-002"));
}

#[test]
fn x83_x84_vat_and_text_complement_rules_are_pairwise() {
    let baseline = v33("83", r#"<Award><VAT>19</VAT></Award>"#);
    let valid = v33(
        "84",
        r#"<Award><VAT>19</VAT><TextComplement MarkLbl="1">filled</TextComplement></Award>"#,
    );
    let report = BusinessValidator::new().validate_pair(&baseline, &valid);
    assert!(!report.has_code("GAEB-LINT-X84-002"));
    assert!(!report.has_code("GAEB-LINT-TEXT-001"));

    let invalid = v33(
        "84",
        r#"<Award><VAT>20</VAT><TextComplement>filled</TextComplement></Award>"#,
    );
    let report = BusinessValidator::new().validate_pair(&baseline, &invalid);
    assert!(report.has_code("GAEB-LINT-X84-002"));
    assert!(report.has_code("GAEB-LINT-TEXT-001"));
    assert!(report.is_valid(), "lints must not invalidate a document");
    assert!(report
        .diagnostics()
        .iter()
        .all(|diagnostic| diagnostic.severity() == ValidationSeverity::Warning));
}

#[test]
fn x31_quantity_links_have_positive_and_negative_cases() {
    let lv = v33(
        "83",
        r#"<Award><BoQ><BoQBody><Itemlist><Item ID="i"><Qty>2.000</Qty></Item></Itemlist></BoQBody></BoQ></Award>"#,
    );
    let valid = v33(
        "31",
        r#"<QtyDeterm IDRef="i"><Result>2.000</Result></QtyDeterm>"#,
    );
    assert_eq!(valid.metadata().phase_code.as_deref(), Some("31"));
    assert_eq!(lv.metadata().phase_code.as_deref(), Some("83"));
    assert!(!BusinessValidator::new()
        .validate_pair(&valid, &lv)
        .has_code("GAEB-LINT-QTY-001"));

    let invalid = v33(
        "31",
        r#"<QtyDeterm IDRef="i"><Result>3.000</Result></QtyDeterm>"#,
    );
    assert!(BusinessValidator::new()
        .validate_pair(&invalid, &lv)
        .has_code("GAEB-LINT-QTY-001"));
}

#[test]
fn x96_x97_order_numbers_have_positive_and_negative_cases() {
    let customer = v33("96", r#"<Order><OrderItem ID="i" RNoPart="10"/></Order>"#);
    let valid = v33("97", r#"<Order><OrderItem ID="i" RNoPart="10"/></Order>"#);
    assert!(!BusinessValidator::new()
        .validate_pair(&customer, &valid)
        .has_code("GAEB-LINT-TRADE-001"));

    let invalid = v33("97", r#"<Order><OrderItem ID="i" RNoPart="11"/></Order>"#);
    assert!(BusinessValidator::new()
        .validate_pair(&customer, &invalid)
        .has_code("GAEB-LINT-TRADE-001"));
}

#[test]
fn pair_lints_require_a_coherent_release_tuple() {
    let baseline = doc(
        "http://www.gaeb.de/GAEB_DA_XML/200407",
        "3.1",
        "2007-06",
        "83",
        r#"<Award><VAT>19</VAT></Award>"#,
    );
    let candidate = v33("84", r#"<Award><VAT>20</VAT></Award>"#);
    assert!(BusinessValidator::new()
        .validate_pair(&baseline, &candidate)
        .diagnostics()
        .is_empty());
}

#[test]
fn x83_x84_detects_description_id_and_breakdown_changes() {
    let baseline = v33(
        "83",
        r#"<Award><BoQ><BoQInfo><BoQBkdn Length="2"><Num>Yes</Num></BoQBkdn></BoQInfo><BoQBody><Itemlist><Item ID="i" RNoPart="10"><Description ID="d1">same</Description></Item></Itemlist></BoQBody></BoQ></Award>"#,
    );
    let candidate = v33(
        "84",
        r#"<Award><BoQ><BoQInfo><BoQBkdn Length="3"><Num>Yes</Num></BoQBkdn></BoQInfo><BoQBody><Itemlist><Item ID="i" RNoPart="10"><Description ID="d2">same</Description></Item></Itemlist></BoQBody></BoQ></Award>"#,
    );
    let report = BusinessValidator::new().validate_pair(&baseline, &candidate);
    assert!(report.has_code("GAEB-LINT-DESCR-001"));
    assert!(report.has_code("GAEB-LINT-BOQ-003"));
}
