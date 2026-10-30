use std::{
    fs,
    path::PathBuf,
};

use ttml_processor::{
    GeneratorConfig,
    generate_ttml,
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

/// 逐行翻译的样本（`type="subtitle"`）在生成后应保持 `subtitle`
#[test]
fn test_line_level_translation_writes_subtitle() {
    let generated = generate_fixture("Red Velvet - Psycho.ttml", &apple_config());

    assert!(
        generated.contains(r#"<translation xml:lang="zh-Hans" type="subtitle">"#),
        "逐行翻译应写入 type=\"subtitle\""
    );
    assert!(
        !generated.contains(r#"type="replacement""#),
        "逐行翻译不应写入 type=\"replacement\""
    );
}

/// 逐字翻译的样本（`type="replacement"`）在生成后应保持 `replacement`
#[test]
fn test_word_level_translation_writes_replacement() {
    let generated = generate_fixture("周杰伦 - 简单爱.ttml", &apple_config());

    assert!(
        generated.contains(r#"<translation xml:lang="zh-Hans" type="replacement">"#),
        "逐字翻译应写入 type=\"replacement\""
    );
    assert!(
        !generated.contains(r#"type="subtitle""#),
        "逐字翻译不应写入 type=\"subtitle\""
    );
}

/// 音译从不写入 `type`，`ttm:name` 始终写入 `type="full"`
#[test]
fn test_transliteration_and_agent_name() {
    let generated = generate_fixture("周杰伦 - 简单爱.ttml", &apple_config());

    assert!(
        generated.contains(r#"<transliteration xml:lang="zh-Latn-pinyin">"#),
        "音译块不应写入 type 属性"
    );
    assert!(
        generated.contains(r#"<ttm:name type="full">周杰倫</ttm:name>"#),
        "ttm:name 应写入 type=\"full\""
    );
}

/// 逐行模式下逐字翻译会被降级为逐行文本，因此写入 `subtitle`
#[test]
fn test_line_timing_downgrades_replacement_to_subtitle() {
    let config = GeneratorConfig {
        use_apple_format_rules: true,
        line_timing: true,
        ..Default::default()
    };

    let generated = generate_fixture("周杰伦 - 简单爱.ttml", &config);

    assert!(
        generated.contains(r#"<translation xml:lang="zh-Hans" type="subtitle">"#),
        "逐行模式下翻译应写入 type=\"subtitle\""
    );
}

/// issue 中的最小示例无法完整复现 `type` 的原始值，但按固定规则应生成合法的类型
#[test]
fn test_issue_example_roundtrip() {
    let xml = r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:itunes="http://music.apple.com/lyric-ttml-internal" xmlns:ttm="http://www.w3.org/ns/ttml#metadata" itunes:timing="Word" xml:lang="ja">
        <head>
            <metadata>
                <ttm:agent type="person" xml:id="v1">
                    <ttm:name type="full">Singer Name</ttm:name>
                </ttm:agent>
                <iTunesMetadata xmlns="http://music.apple.com/lyric-ttml-internal">
                    <translations>
                        <translation type="subtitle" xml:lang="zh-Hans">
                            <text for="L1">中文翻译</text>
                        </translation>
                    </translations>
                    <transliterations>
                        <transliteration type="replacement" xml:lang="ja-Latn">
                            <text for="L1">romaji</text>
                        </transliteration>
                    </transliterations>
                </iTunesMetadata>
            </metadata>
        </head>
        <body>
            <div>
                <p begin="0.000" end="1.000" itunes:key="L1" ttm:agent="v1">日本語</p>
            </div>
        </body>
    </tt>"#;

    let parsed = parse_ttml(xml).unwrap();
    let generated = generate_ttml(&parsed, &apple_config()).unwrap();

    assert!(
        generated.contains(r#"<ttm:name type="full">Singer Name</ttm:name>"#),
        "ttm:name 应写入 type=\"full\""
    );
    assert!(
        generated.contains(r#"<translation xml:lang="zh-Hans" type="subtitle">"#),
        "逐行翻译应写入 type=\"subtitle\""
    );
    assert!(
        generated.contains(r#"<transliteration xml:lang="ja-Latn">"#),
        "音译块不应写入 type 属性"
    );
    assert!(
        !generated.contains(r#"role="x-translation""#),
        "启用 Apple 格式规则时翻译不应内嵌到歌词行"
    );
}
