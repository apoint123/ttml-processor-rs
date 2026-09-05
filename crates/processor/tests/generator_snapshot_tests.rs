use std::fs;

use insta::assert_snapshot;
use serde::Serialize;
use serde_json::ser::{
    PrettyFormatter,
    Serializer,
};
use ttml_processor::{
    GeneratorConfig,
    generate_ttml,
    parse_ttml,
};

#[allow(clippy::missing_errors_doc)]
#[allow(clippy::missing_panics_doc)]
pub fn to_string_tab_indent<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let mut buf = Vec::new();

    let formatter = PrettyFormatter::with_indent(b"\t");
    let mut ser = Serializer::with_formatter(&mut buf, formatter);
    value.serialize(&mut ser)?;

    Ok(String::from_utf8(buf).unwrap())
}

#[test]
fn test_roundtrip() {
    let config = GeneratorConfig {
        use_apple_format_rules: true,
        ..Default::default()
    };

    insta::glob!("fixtures/*.ttml", |path| {
        let xml_content = fs::read_to_string(path).unwrap();
        let parsed = parse_ttml(&xml_content).unwrap();

        let generated_xml = generate_ttml(&parsed, &config).unwrap();
        let reparsed = parse_ttml(&generated_xml).unwrap();

        let json_string = to_string_tab_indent(&reparsed).unwrap();

        assert_snapshot!(json_string);
    });
}

#[test]
fn test_generate_ttml() {
    let config = GeneratorConfig {
        use_apple_format_rules: true,
        ..Default::default()
    };

    insta::glob!("fixtures/*.ttml", |path| {
        let xml_content = fs::read_to_string(path).unwrap();
        let parsed = parse_ttml(&xml_content).unwrap();

        let generated_xml = generate_ttml(&parsed, &config).unwrap();

        assert_snapshot!(generated_xml);
    });
}

#[test]
fn test_generate_ttml_formatted() {
    let config = GeneratorConfig {
        use_apple_format_rules: true,
        format: true,
    };

    insta::glob!("fixtures/*.ttml", |path| {
        let xml_content = fs::read_to_string(path).unwrap();
        let parsed = parse_ttml(&xml_content).unwrap();

        let generated_xml = generate_ttml(&parsed, &config).unwrap();

        assert_snapshot!(generated_xml);
    });
}

#[test]
fn test_roundtrip_formatted() {
    let config = GeneratorConfig {
        use_apple_format_rules: true,
        format: true,
    };

    insta::glob!("fixtures/*.ttml", |path| {
        let xml_content = fs::read_to_string(path).unwrap();
        let parsed = parse_ttml(&xml_content).unwrap();

        let generated_xml = generate_ttml(&parsed, &config).unwrap();
        let reparsed = parse_ttml(&generated_xml).unwrap();

        let json_string = to_string_tab_indent(&reparsed).unwrap();

        assert_snapshot!(json_string);
    });
}
