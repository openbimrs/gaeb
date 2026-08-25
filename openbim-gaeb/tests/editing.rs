use openbim_gaeb::{Document, Error};

fn fixture() -> Vec<u8> {
    br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"><GAEBInfo><Version>3.3</Version></GAEBInfo><Award><DP>86</DP><BoQ><BoQBody><Itemlist><Item ID="item-1" RNoPart="10"><Qty>1.000</Qty><QU>m3</QU><vendor>keep me</vendor></Item></Itemlist></BoQBody></BoQ></Award></GAEB>"#.to_vec()
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

    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA84/3.3"><GAEBInfo><Version>3.3</Version></GAEBInfo><Award><DP>84</DP><Item ID="priced"><UP>1.0</UP></Item></Award></GAEB>"#;
    let mut document = Document::parse(xml).unwrap();
    assert!(matches!(
        document.set_item_quantity("priced", "1"),
        Err(Error::QuantityMissing(_))
    ));
}

#[test]
fn duplicate_item_ids_are_not_edited_ambiguously() {
    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"><GAEBInfo><Version>3.3</Version></GAEBInfo><Award><DP>86</DP><Item ID="dup"><Qty>1</Qty></Item><Item ID="dup"><Qty>2</Qty></Item></Award></GAEB>"#;
    let mut document = Document::parse(xml).unwrap();
    assert!(matches!(
        document.set_item_quantity("dup", "3"),
        Err(Error::AmbiguousItem(_))
    ));
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
