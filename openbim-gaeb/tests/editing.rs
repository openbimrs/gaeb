use openbim_gaeb::{Document, Error};

fn fixture() -> Vec<u8> {
    br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"><GAEBInfo><Version>3.3</Version></GAEBInfo><Award><DP>86</DP><BoQ><BoQBody><Itemlist><Item ID="item-1" RNoPart="10"><Qty>1.000</Qty><QU>m3</QU><vendor>keep me</vendor></Item></Itemlist></BoQBody></BoQ></Award></GAEB>"#.to_vec()
}

fn schema_items(phase: &str, items: &str) -> Vec<u8> {
    format!(
        r#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA{phase}/3.3"><GAEBInfo><Version>3.3</Version></GAEBInfo><Award><DP>{phase}</DP><BoQ><BoQBody><Itemlist>{items}</Itemlist></BoQBody></BoQ></Award></GAEB>"#
    )
    .into_bytes()
}

#[test]
fn quantity_edit_changes_only_the_target_text_and_reparses() {
    let before = fixture();
    let mut document = Document::parse(&before).unwrap();
    document.set_item_quantity("item-1", "2.750").unwrap();

    let expected = String::from_utf8(before)
        .unwrap()
        .replace("<Qty>1.000</Qty>", "<Qty>2.750</Qty>");
    assert_eq!(document.as_bytes(), expected.as_bytes());
    assert_eq!(
        document.item("item-1").unwrap().quantity.as_deref(),
        Some("2.750")
    );
    assert!(String::from_utf8_lossy(document.as_bytes()).contains("<vendor>keep me</vendor>"));
}

#[test]
fn quantity_edit_rejects_non_xsd_decimal_lexemes_without_mutation() {
    let before = fixture();
    let mut document = Document::parse(&before).unwrap();
    assert!(matches!(
        document.set_item_quantity("item-1", "1e3"),
        Err(Error::InvalidDecimal(_))
    ));
    assert_eq!(document.as_bytes(), before);
}

#[test]
fn quantity_edit_distinguishes_missing_item_and_missing_quantity() {
    let mut document = Document::parse(fixture()).unwrap();
    assert!(matches!(
        document.set_item_quantity("absent", "1"),
        Err(Error::ItemNotFound(_))
    ));

    let xml = schema_items("84", r#"<Item ID="priced"><UP>1.0</UP></Item>"#);
    let mut document = Document::parse(&xml).unwrap();
    assert!(matches!(
        document.set_item_quantity("priced", "1"),
        Err(Error::QuantityMissing(_))
    ));
}

#[test]
fn nested_item_fields_do_not_attach_to_the_active_schema_item() {
    let xml = schema_items(
        "86",
        r#"<Item ID="outer"><Item ID="inner"><Qty>7</Qty><Description><p>nested</p></Description></Item><QU>m</QU></Item>"#,
    );
    let mut document = Document::parse(&xml).unwrap();
    let outer = document.item("outer").unwrap();
    assert_eq!(outer.quantity, None);
    assert_eq!(outer.description, None);
    assert_eq!(outer.unit.as_deref(), Some("m"));
    assert!(document.item("inner").is_none());
    assert!(matches!(
        document.set_item_quantity("outer", "8"),
        Err(Error::QuantityMissing(_))
    ));
    assert_eq!(document.as_bytes(), xml);
}

#[test]
fn empty_quantity_is_existing_but_not_editable() {
    let xml = schema_items("86", r#"<Item ID="empty"><Qty/></Item>"#);
    let mut document = Document::parse(&xml).unwrap();
    assert_eq!(document.item("empty").unwrap().quantity, None);
    assert!(matches!(
        document.set_item_quantity("empty", "8"),
        Err(Error::QuantityNotEditable(_))
    ));
    assert_eq!(document.as_bytes(), xml);
}

#[test]
fn duplicate_item_ids_are_not_edited_ambiguously() {
    let xml = schema_items(
        "86",
        r#"<Item ID="dup"><Qty>1</Qty></Item><Item ID="dup"><Qty>2</Qty></Item>"#,
    );
    let mut document = Document::parse(&xml).unwrap();
    assert!(matches!(
        document.set_item_quantity("dup", "3"),
        Err(Error::AmbiguousItem(_))
    ));
}

#[test]
fn quantity_entities_are_read_completely_and_replaced_as_one_value() {
    let xml = schema_items("86", r#"<Item ID="entity"><Qty>1&#x2E;5</Qty></Item>"#);
    let mut document = Document::parse(&xml).unwrap();
    assert_eq!(
        document.item("entity").unwrap().quantity.as_deref(),
        Some("1.5")
    );

    document.set_item_quantity("entity", "9").unwrap();
    assert!(String::from_utf8_lossy(document.as_bytes()).contains("<Qty>9</Qty>"));
}

#[test]
fn quantity_comments_are_read_completely_but_edits_fail_closed() {
    let xml = schema_items(
        "86",
        r#"<Item ID="split"><Qty>1<!-- preserve -->.5</Qty></Item>"#,
    );
    let mut document = Document::parse(&xml).unwrap();
    let before = document.as_bytes().to_vec();
    assert_eq!(
        document.item("split").unwrap().quantity.as_deref(),
        Some("1.5")
    );

    assert!(matches!(
        document.set_item_quantity("split", "9"),
        Err(Error::QuantityNotEditable(id)) if id == "split"
    ));
    assert_eq!(document.as_bytes(), before);
}

#[test]
fn cdata_quantity_is_read_and_edited_inside_the_cdata_section() {
    let xml = schema_items(
        "86",
        r#"<Item ID="cdata"><Qty><![CDATA[1.5]]></Qty></Item>"#,
    );
    let mut document = Document::parse(&xml).unwrap();
    assert_eq!(
        document.item("cdata").unwrap().quantity.as_deref(),
        Some("1.5")
    );

    document.set_item_quantity("cdata", "9").unwrap();
    assert!(String::from_utf8_lossy(document.as_bytes()).contains("<Qty><![CDATA[9]]></Qty>"));
}

#[test]
fn missing_item_ids_cannot_be_used_as_mutation_handles() {
    let xml = schema_items("86", "<Item><Qty>1</Qty></Item>");
    let mut document = Document::parse(&xml).unwrap();
    let before = document.as_bytes().to_vec();

    assert!(document.item("").is_none());
    assert!(matches!(
        document.set_item_quantity("", "2"),
        Err(Error::ItemNotFound(id)) if id.is_empty()
    ));
    assert_eq!(document.as_bytes(), before);
}

#[test]
fn nested_quantity_markup_is_not_exposed_as_a_fabricated_value() {
    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3"><Award><BoQ><BoQBody><Itemlist><Item ID="nested"><Qty>1<X>2</X>3</Qty></Item></Itemlist></BoQBody></BoQ></Award></GAEB>"#;
    let mut document = Document::parse(xml).unwrap();
    assert_eq!(document.item("nested").unwrap().quantity, None);
    assert!(matches!(
        document.set_item_quantity("nested", "9"),
        Err(Error::QuantityNotEditable(id)) if id == "nested"
    ));
}

#[test]
fn duplicate_quantity_elements_are_not_edited_ambiguously() {
    let xml = schema_items(
        "86",
        r#"<Item ID="dup-qty"><Qty>1</Qty><Qty>2</Qty></Item>"#,
    );
    let mut document = Document::parse(&xml).unwrap();
    let before = document.as_bytes().to_vec();
    assert_eq!(document.item("dup-qty").unwrap().quantity, None);

    assert!(matches!(
        document.set_item_quantity("dup-qty", "3"),
        Err(Error::QuantityNotEditable(id)) if id == "dup-qty"
    ));
    assert_eq!(document.as_bytes(), before);
}

#[test]
fn quantity_edit_accounts_for_a_preserved_utf8_bom() {
    let mut bytes = b"\xEF\xBB\xBF".to_vec();
    bytes.extend_from_slice(&fixture());
    let mut document = Document::parse(&bytes).unwrap();
    document.set_item_quantity("item-1", "9").unwrap();

    assert!(document.as_bytes().starts_with(b"\xEF\xBB\xBF"));
    let xml = String::from_utf8_lossy(document.as_bytes());
    assert!(xml.contains("<Qty>9</Qty>"));
    assert!(!xml.contains("<Qty>1.000</Qty>"));
}
