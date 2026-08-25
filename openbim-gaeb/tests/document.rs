use openbim_gaeb::{Document, Error};

const XML_BODY: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<!-- vendor extension must survive -->
<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3" xmlns:vendor="urn:vendor">
  <GAEBInfo><Version>3.3</Version><VersDate>2023-01</VersDate></GAEBInfo>
  <Award><DP>83</DP><BoQ><BoQBody>
    <BoQCtgy ID="cat-1" RNoPart="01"><LblTx>Earthworks</LblTx><Itemlist>
      <Item ID="item-1" RNoPart="0010"><Qty>1.000</Qty><QU>m3</QU><UP>12.50</UP><IT>12.50</IT><Description><CompleteText><Text><p><span>Excavate &amp; haul</span></p></Text></CompleteText></Description><vendor:opaque answer="42"/></Item>
    </Itemlist></BoQCtgy>
  </BoQBody></BoQ></Award>
</GAEB>
"#;

fn xml() -> Vec<u8> {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(XML_BODY);
    bytes
}

#[test]
fn extracts_common_boq_item_view() {
    let bytes = xml();
    let document = Document::parse(&bytes).unwrap();
    let item = document.item("item-1").unwrap();
    assert_eq!(item.outline_number.as_deref(), Some("0010"));
    assert_eq!(item.quantity.as_deref(), Some("1.000"));
    assert_eq!(item.unit.as_deref(), Some("m3"));
    assert_eq!(item.unit_price.as_deref(), Some("12.50"));
    assert_eq!(item.total_price.as_deref(), Some("12.50"));
    assert_eq!(item.description.as_deref(), Some("Excavate & haul"));
    assert_eq!(item.category_path.len(), 1);
    assert_eq!(item.category_path[0].id.as_deref(), Some("cat-1"));
    assert_eq!(item.category_path[0].label.as_deref(), Some("Earthworks"));
}

#[test]
fn unchanged_write_is_byte_identical_including_bom_and_unknown_xml() {
    let bytes = xml();
    let document = Document::parse(&bytes).unwrap();
    assert_eq!(document.as_bytes(), bytes);
    let mut output = Vec::new();
    document.write_to(&mut output).unwrap();
    assert_eq!(output, bytes);
    assert_eq!(document.into_bytes(), bytes);
}

#[test]
fn rejects_non_xml_and_non_gaeb_documents() {
    assert!(matches!(Document::parse(b"PK\x03\x04"), Err(Error::NotXml)));
    assert!(matches!(Document::parse(b"<root/>"), Err(Error::NotGaeb)));
}

#[test]
fn ignores_vendor_elements_that_reuse_gaeb_local_names() {
    let xml = br#"<g:GAEB xmlns:g="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3" xmlns:v="urn:vendor"><g:GAEBInfo><g:Version>3.3</g:Version><v:Version>9.9</v:Version></g:GAEBInfo><g:Award><g:DP>86</g:DP><v:DP>99</v:DP><v:Item ID="spoof"><v:Qty>999</v:Qty></v:Item><g:Item ID="real"><v:Qty>888</v:Qty><g:Qty>1.5</g:Qty><v:Description>spoof</v:Description></g:Item></g:Award></g:GAEB>"#;
    let document = Document::parse(xml).unwrap();

    assert_eq!(document.items().len(), 1);
    assert!(document.item("spoof").is_none());
    let item = document.item("real").unwrap();
    assert_eq!(item.quantity.as_deref(), Some("1.5"));
    assert_eq!(item.description, None);
    assert_eq!(document.metadata().version_text.as_deref(), Some("3.3"));
    assert_eq!(document.metadata().phase_code.as_deref(), Some("86"));
}

#[test]
fn namespaced_vendor_id_does_not_become_a_gaeb_item_id() {
    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3" xmlns:v="urn:vendor"><GAEBInfo><Version>3.3</Version></GAEBInfo><Award><DP>86</DP><Item v:ID="spoof"><Qty>1</Qty></Item></Award></GAEB>"#;
    let document = Document::parse(xml).unwrap();

    assert_eq!(document.items().len(), 1);
    assert_eq!(document.items()[0].id, "");
    assert!(document.item("spoof").is_none());
}

#[test]
fn rejects_spoofed_namespaces_and_extra_root_elements() {
    let spoof = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/not-gaeb"><GAEBInfo><Version>3.3</Version></GAEBInfo></GAEB>"#;
    assert!(matches!(Document::parse(spoof), Err(Error::NotGaeb)));

    let multiple = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3"><GAEBInfo><Version>3.3</Version></GAEBInfo></GAEB><vendor/>"#;
    assert!(matches!(Document::parse(multiple), Err(Error::Xml(_))));

    let trailing_text = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3"><GAEBInfo><Version>3.3</Version></GAEBInfo></GAEB>not-xml"#;
    assert!(matches!(Document::parse(trailing_text), Err(Error::Xml(_))));
}

#[test]
fn rejects_undeclared_namespace_prefixes() {
    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3"><GAEBInfo><Version>3.3</Version></GAEBInfo><x:Item ID="bad"/></GAEB>"#;
    assert!(matches!(Document::parse(xml), Err(Error::Xml(_))));
}

#[test]
fn malformed_xml_is_an_error() {
    assert!(matches!(
        Document::parse(br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3">"#),
        Err(Error::Xml(_))
    ));
}
