use ttml_processor::{
    error::{
        ParseErrorKind,
        TTMLProcessorError,
    },
    parse_ttml,
};

#[test]
fn test_error_context_invalid_timestamp() {
    let xml = r#"<tt xmlns="http://www.w3.org/ns/ttml" xml:lang="zh">
            <body>
                <div itunes:songPart="Verse">
                    <p begin="0.500" end="1.000" itunes:key="L1" ttm:agent="v1">
                        <span begin="invalid_time" end="1.000">test</span>
                    </p>
                </div>
            </body>
        </tt>"#;

    let result = parse_ttml(xml);

    let Err(TTMLProcessorError::ParseError { kind, context }) = result else {
        panic!("Expected ParseError, got {result:#?}");
    };

    match kind {
        ParseErrorKind::InvalidTimestamp(val) => assert_eq!(val, "invalid_time"),
        _ => panic!("Expected InvalidTimestamp, got {kind:?}"),
    }

    assert_eq!(context.line_id.as_deref(), Some("L1"));
    assert_eq!(context.tag_stack, vec!["tt", "body", "div", "p", "span"]);
    assert_eq!(context.current_attribute.as_deref(), Some("begin"));
    assert!(context.byte_offset > 50);
}

#[test]
fn test_error_context_missing_attribute() {
    let xml = r#"<tt xmlns="http://www.w3.org/ns/ttml">
            <head>
                <metadata>
                    <ttm:agent type="person" /> 
                </metadata>
            </head>
        </tt>"#;

    let result = parse_ttml(xml);

    let Err(TTMLProcessorError::ParseError { kind, context }) = result else {
        panic!("Expected ParseError, got {result:#?}");
    };

    match kind {
        ParseErrorKind::MissingAttribute(val) => assert_eq!(val, "xml:id"),
        _ => panic!("Expected MissingAttribute, got {kind:?}"),
    }

    assert_eq!(context.line_id, None);
    assert_eq!(
        context.tag_stack,
        vec!["tt", "head", "metadata", "ttm:agent"]
    );
    assert_eq!(context.current_attribute.as_deref(), Some("xml:id"));
}

#[test]
fn test_error_context_unexpected_eof() {
    let xml = r#"<tt xmlns="http://www.w3.org/ns/ttml">
            <body>
                <div itunes:songPart="Chorus">
                    <p begin="0.0" end="1.0" itunes:key="L2">
                        <span begin="0.0" end="1.0">Unfinished"#;

    let result = parse_ttml(xml);

    let Err(TTMLProcessorError::ParseError { kind, context }) = result else {
        panic!("Expected ParseError, got {result:#?}");
    };

    match kind {
        ParseErrorKind::UnexpectedEof => (),
        _ => panic!("Expected UnexpectedEof, got {kind:?}"),
    }

    assert_eq!(context.line_id.as_deref(), Some("L2"));
    assert_eq!(context.tag_stack, vec!["tt", "body", "div", "p", "span"]);
    assert_eq!(context.current_attribute, None);
}
