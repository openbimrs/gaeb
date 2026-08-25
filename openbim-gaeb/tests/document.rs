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
fn extracts_only_schema_positioned_boq_items_and_categories() {
    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3">
      <GAEBInfo><Item ID="header"><Qty>999</Qty></Item></GAEBInfo>
      <Award>
        <BoQCtgy ID="invalid"><Itemlist><Item ID="wrong-category"><Qty>5</Qty></Item></Itemlist></BoQCtgy>
        <BoQ><BoQBody><BoQCtgy ID="valid"><LblTx>Works</LblTx><BoQBody><Itemlist>
          <Item ID="real"><Qty>1</Qty></Item>
        </Itemlist></BoQBody></BoQCtgy></BoQBody></BoQ>
      </Award>
    </GAEB>"#;

    let document = Document::parse(xml).unwrap();
    let ids: Vec<_> = document
        .items()
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    assert_eq!(ids, ["wrong-category", "real"]);
    assert!(document.item("header").is_none());
    assert!(document
        .item("wrong-category")
        .unwrap()
        .category_path
        .is_empty());
    assert_eq!(
        document.item("real").unwrap().category_path[0]
            .label
            .as_deref(),
        Some("Works")
    );
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
    let xml = br#"<g:GAEB xmlns:g="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3" xmlns:v="urn:vendor"><g:GAEBInfo><g:Version>3.3</g:Version><v:Version>9.9</v:Version></g:GAEBInfo><g:Award><g:DP>86</g:DP><g:BoQ><g:BoQBody><g:Itemlist><v:Item ID="spoof"><v:Qty>999</v:Qty></v:Item><g:Item ID="real"><v:Qty>888</v:Qty><g:Qty>1.5</g:Qty><v:Description>spoof</v:Description></g:Item></g:Itemlist></g:BoQBody></g:BoQ></g:Award></g:GAEB>"#;
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
    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3" xmlns:v="urn:vendor"><GAEBInfo><Version>3.3</Version></GAEBInfo><Award><DP>86</DP><BoQ><BoQBody><Itemlist><Item v:ID="spoof"><Qty>1</Qty></Item></Itemlist></BoQBody></BoQ></Award></GAEB>"#;
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
fn rejects_duplicate_attributes_instead_of_selecting_one() {
    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"><Award><Item ID="a" ID="b"><Qty>1</Qty></Item></Award></GAEB>"#;
    assert!(matches!(Document::parse(xml), Err(Error::Xml(_))));
}

#[test]
fn rejects_misplaced_or_repeated_xml_declarations() {
    let nested =
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"><?xml version="1.0"?></GAEB>"#;
    assert!(matches!(Document::parse(nested), Err(Error::Xml(_))));

    let repeated = br#"<?xml version="1.0"?><?xml version="1.0"?><GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"/>"#;
    assert!(matches!(Document::parse(repeated), Err(Error::Xml(_))));

    let after_comment = br#"<!-- prolog comment --><?xml version="1.0"?><GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"/>"#;
    assert!(matches!(Document::parse(after_comment), Err(Error::Xml(_))));
}

#[test]
fn rejects_malformed_utf8_even_in_uninterpreted_xml() {
    for mut xml in [
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"><!-- ~ --></GAEB>"#.to_vec(),
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"><?vendor ~?></GAEB>"#.to_vec(),
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3" vendor="~"/>"#.to_vec(),
    ] {
        let index = xml.iter().position(|byte| *byte == b'~').unwrap();
        xml[index] = 0xff;
        assert!(matches!(Document::parse(xml), Err(Error::Xml(_))));
    }
}

#[test]
fn validates_every_attribute_namespace_and_entity() {
    let undeclared = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3" bad:x="1"/>"#;
    assert!(matches!(Document::parse(undeclared), Err(Error::Xml(_))));

    let unknown_entity =
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3" vendor="&unknown;"/>"#;
    assert!(matches!(
        Document::parse(unknown_entity),
        Err(Error::Xml(_))
    ));

    let unknown_text_entity = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"><Vendor>&unknown;</Vendor></GAEB>"#;
    assert!(matches!(
        Document::parse(unknown_text_entity),
        Err(Error::Xml(_))
    ));
}

#[test]
fn rejects_unsupported_xml_declarations() {
    for declaration in [
        r#"<?xml version="1.1"?>"#,
        r#"<?xml version="1.0" encoding="ISO-8859-1"?>"#,
        r#"<?xml version="1.0" standalone="maybe"?>"#,
    ] {
        let xml =
            format!(r#"{declaration}<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"/>"#);
        assert!(matches!(Document::parse(xml), Err(Error::Xml(_))));
    }
}

#[test]
fn rejects_xml_1_0_forbidden_character_references() {
    for entity in ["&#x1;", "&#xB;"] {
        let xml = format!(
            r#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"><Award><Item ID="i"><Qty>{entity}</Qty></Item></Award></GAEB>"#
        );
        assert!(matches!(
            Document::parse(xml.as_bytes()),
            Err(Error::Xml(_))
        ));
    }
}

#[test]
fn item_description_excludes_nested_subdescription_text() {
    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"><Award><DP>86</DP><BoQ><BoQBody><Itemlist><Item ID="i"><Description><CompleteText><Text><p>parent</p></Text></CompleteText></Description><SubDescr><Description><CompleteText><Text><p>nested</p></Text></CompleteText></Description></SubDescr></Item></Itemlist></BoQBody></BoQ></Award></GAEB>"#;
    let document = Document::parse(xml).unwrap();
    assert_eq!(
        document.item("i").unwrap().description.as_deref(),
        Some("parent")
    );
}

#[test]
fn metadata_capture_requires_the_official_structural_parent() {
    let xml = br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"><GAEBInfo><Version>3.3</Version><Nested><Version>9.9</Version></Nested></GAEBInfo><Award><DP>86</DP><Nested><DP>99</DP></Nested></Award></GAEB>"#;
    let document = Document::parse(xml).unwrap();
    assert_eq!(document.metadata().version_text.as_deref(), Some("3.3"));
    assert_eq!(document.metadata().phase_code.as_deref(), Some("86"));
}

#[test]
fn rejects_invalid_xml_element_names() {
    let xml = br#"<1GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"/>"#;
    assert!(matches!(Document::parse(xml), Err(Error::Xml(_))));
}

#[test]
fn rejects_other_xml_lexical_malformations() {
    for xml in [
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3" 1bad="x"/>"#.as_slice(),
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"><!-- a--b --></GAEB>"#.as_slice(),
        br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3" xmlns:v="one" xmlns:v="two"/>"#
            .as_slice(),
        br#"<?xml version="1.0" vendor="x"?><GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"/>"#
            .as_slice(),
        br#"<?xml encoding="UTF-8" version="1.0"?><GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"/>"#
            .as_slice(),
        br#"<?xml version="1.0" version="1.0"?><GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"/>"#
            .as_slice(),
    ] {
        let result = Document::parse(xml);
        assert!(
            matches!(result, Err(Error::Xml(_))),
            "malformed input was not rejected as XML: {result:?}"
        );
    }
}

#[test]
fn local_byte_corruptions_and_truncations_return_without_panicking() {
    let seed = br#"<?xml version="1.0"?><GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA86/3.3"><GAEBInfo><Version>3.3</Version></GAEBInfo><Award><DP>86</DP><Item ID="i"><Qty><![CDATA[1.5]]></Qty></Item></Award></GAEB>"#;

    for end in 0..seed.len() {
        let _ = Document::parse(&seed[..end]);
    }
    for index in 0..seed.len() {
        for replacement in [0, b'<', b'&', 0xff] {
            let mut candidate = seed.to_vec();
            candidate[index] = replacement;
            let _ = Document::parse(candidate);
        }
    }
}

#[test]
fn malformed_xml_is_an_error() {
    assert!(matches!(
        Document::parse(br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3">"#),
        Err(Error::Xml(_))
    ));
}
