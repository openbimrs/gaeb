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
fn malformed_xml_is_an_error() {
    assert!(matches!(
        Document::parse(br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3">"#),
        Err(Error::Xml(_))
    ));
}
