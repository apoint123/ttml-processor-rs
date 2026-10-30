use std::{
    fs,
    path::PathBuf,
};

use ttml_processor::{
    GeneratorConfig,
    generate_ttml,
    model::TTMLResult,
    parse_ttml,
};

fn apple_config() -> GeneratorConfig {
    GeneratorConfig {
        use_apple_format_rules: true,
        ..Default::default()
    }
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn generate_fixture(name: &str, config: &GeneratorConfig) -> String {
    let xml_content = fs::read_to_string(fixture_path(name)).unwrap();
    let parsed = parse_ttml(&xml_content).unwrap();
    generate_ttml(&parsed, config).unwrap()
}

/// 构造一份带可选 `automaticallyCreated` 的翻译样本
fn synthetic_ttml(value: Option<&str>) -> String {
    let attr = value.map_or_else(String::new, |v| format!(r#" automaticallyCreated="{v}""#));

    format!(
        r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:itunes="http://music.apple.com/lyric-ttml-internal" xmlns:ttm="http://www.w3.org/ns/ttml#metadata" itunes:timing="Word" xml:lang="ja">
            <head>
                <metadata>
                    <iTunesMetadata xmlns="http://music.apple.com/lyric-ttml-internal">
                        <translations>
                            <translation{attr} xml:lang="zh-Hans">
                                <text for="L1">中文翻译</text>
                            </translation>
                        </translations>
                    </iTunesMetadata>
                </metadata>
            </head>
            <body>
                <div>
                    <p begin="0.000" end="1.000" itunes:key="L1">日本語</p>
                </div>
            </body>
        </tt>"#
    )
}

fn parse_synthetic(value: Option<&str>) -> TTMLResult {
    parse_ttml(&synthetic_ttml(value)).unwrap()
}

fn translation_flag(result: &TTMLResult) -> Option<bool> {
    result.lines[0].translations.as_ref().unwrap()[0].automatically_created
}

/// 真实样本：逐字音译上的 `automaticallyCreated="true"` 应保留
#[test]
fn test_transliteration_flag_roundtrip() {
    let mut config = apple_config();
    config.format = false;

    let generated = generate_fixture("Red Velvet - Psycho.ttml", &config);
    assert!(
        generated.contains(r#"<transliteration automaticallyCreated="true" xml:lang="ko-Latn">"#),
        "音译块应保留 automaticallyCreated 属性"
    );

    let reparsed = parse_ttml(&generated).unwrap();
    let flag = reparsed
        .lines
        .iter()
        .flat_map(|line| line.romanizations.as_deref().unwrap_or_default())
        .next()
        .and_then(|content| content.automatically_created);
    assert_eq!(flag, Some(true));

    // 该文件中的翻译块本来就没有该属性，不应被无中生有
    assert!(
        !generated.contains("<translation automaticallyCreated"),
        "没有该属性的翻译块不应被写入"
    );
}

/// 合成样本：`<translation>` 上的标志同样应往返保留
#[test]
fn test_translation_flag_roundtrip() {
    let parsed = parse_synthetic(Some("true"));
    assert_eq!(translation_flag(&parsed), Some(true));

    let generated = generate_ttml(&parsed, &apple_config()).unwrap();
    assert!(
        generated.contains(r#"<translation automaticallyCreated="true" xml:lang="zh-Hans""#),
        "翻译块应保留 automaticallyCreated 属性"
    );
}

/// `true` / `1` 视为自动生成；`false` / `0` / 未知值与缺失等价
#[test]
fn test_value_semantics() {
    for value in ["true", "1"] {
        let parsed = parse_synthetic(Some(value));
        assert_eq!(translation_flag(&parsed), Some(true), "value = {value}");

        let generated = generate_ttml(&parsed, &apple_config()).unwrap();
        assert!(generated.contains(r#"automaticallyCreated="true""#));
    }

    for value in ["false", "0", "auto"] {
        let parsed = parse_synthetic(Some(value));
        assert_eq!(translation_flag(&parsed), None, "value = {value}");

        let generated = generate_ttml(&parsed, &apple_config()).unwrap();
        assert!(
            !generated.contains("automaticallyCreated"),
            "value = {value}"
        );
    }

    let parsed = parse_synthetic(None);
    assert_eq!(translation_flag(&parsed), None);
    let generated = generate_ttml(&parsed, &apple_config()).unwrap();
    assert!(!generated.contains("automaticallyCreated"));
}

/// 调用者构造的 `Some(false)` 在生成时视为缺失
#[test]
fn test_some_false_is_not_written() {
    let mut parsed = parse_synthetic(Some("true"));
    parsed.lines[0].translations.as_mut().unwrap()[0].automatically_created = Some(false);

    let generated = generate_ttml(&parsed, &apple_config()).unwrap();
    assert!(!generated.contains("automaticallyCreated"));
}

/// 内嵌输出无法承载该标志：逐字音译在 `line_timing` + 非 Apple 规则下会丢失
#[test]
fn test_inline_output_drops_flag() {
    let inline_config = GeneratorConfig {
        use_apple_format_rules: false,
        line_timing: true,
        ..Default::default()
    };

    let generated = generate_fixture("Red Velvet - Psycho.ttml", &inline_config);
    assert!(generated.contains(r#"role="x-roman""#), "应走内嵌输出路径");
    assert!(!generated.contains("automaticallyCreated"));

    // 默认配置下逐字音译仍写入 head，标志得以保留
    let default_generated =
        generate_fixture("Red Velvet - Psycho.ttml", &GeneratorConfig::default());
    assert!(default_generated.contains(r#"automaticallyCreated="true""#));
}
