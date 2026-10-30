use std::fs;

use insta::assert_snapshot;
use serde::Serialize;
use serde_json::ser::{
    PrettyFormatter,
    Serializer,
};
use ttml_processor::parse_ttml;

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
fn test_lyric_parsing() {
    insta::glob!("fixtures/*.ttml", |path| {
        let xml_content = fs::read_to_string(path).unwrap();
        let result = parse_ttml(&xml_content).unwrap();

        let json_string = to_string_tab_indent(&result).unwrap();

        assert_snapshot!(json_string);
    });
}
