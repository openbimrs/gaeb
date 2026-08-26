use openbim_gaeb_bindings::v3_1_2007_11::{
    TgDecimal113Type, TgDecimal132Type, TgDecimal133Type, TgDecimal52Type, TgDecimal64Type,
    TgDecimalType,
};

#[test]
fn all_gaeb_decimal_aliases_preserve_values_beyond_binary64_integer_precision() {
    let lexical = "9007199254740993.01";
    macro_rules! assert_exact {
        ($decimal:ty) => {{
            let value: $decimal = lexical.parse().unwrap();
            assert_eq!(value.to_string(), lexical);
        }};
    }
    assert_exact!(TgDecimalType);
    assert_exact!(TgDecimal113Type);
    assert_exact!(TgDecimal132Type);
    assert_exact!(TgDecimal133Type);
    assert_exact!(TgDecimal52Type);
    assert_exact!(TgDecimal64Type);
}

#[test]
fn generated_serializer_qualifies_xml_space() {
    let source = include_str!("../src/generated/v3_1_2007_11.rs");
    assert!(source.contains("write_attrib(&mut bytes, \"xml:space\""));
    assert!(!source.contains("write_attrib(&mut bytes, \"space\""));
}
