use std::{
    fs,
    path::PathBuf,
};

use ttml_processor::{
    GeneratorConfig,
    amll::to_amll_lyrics,
    generate_ttml,
    model::{
        AudioMetadata,
        LyricLine,
        TTMLMetadata,
        TTMLResult,
    },
    parse_ttml,
};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn parse_fixture(name: &str) -> TTMLResult {
    let xml_content = fs::read_to_string(fixture_path(name)).unwrap();
    parse_ttml(&xml_content).unwrap()
}

fn flat_config() -> GeneratorConfig {
    GeneratorConfig {
        format: false,
        ..Default::default()
    }
}

const fn audio_metadata(result: &TTMLResult) -> &AudioMetadata {
    result.metadata.audio.as_ref().expect("应解析出音频元数据")
}

fn synthetic_ttml(head: &str, body_attrs: &str) -> String {
    format!(
        r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:itunes="http://music.apple.com/lyric-ttml-internal" xmlns:ttm="http://www.w3.org/ns/ttml#metadata" itunes:timing="Word" xml:lang="en">
            <head>
                <metadata>
                    {head}
                </metadata>
            </head>
            <body{body_attrs}>
                <div>
                    <p begin="0.000" end="1.000" itunes:key="L1">Hello</p>
                </div>
            </body>
        </tt>"#
    )
}

fn sample_result(audio: Option<AudioMetadata>) -> TTMLResult {
    TTMLResult {
        metadata: TTMLMetadata {
            language: Some("en".into()),
            audio,
            ..Default::default()
        },
        lines: vec![LyricLine {
            text: "Hello".into(),
            start_time: 1000,
            end_time: 2000,
            ..Default::default()
        }],
    }
}

#[test]
fn test_parse_audio_metadata_from_duet() {
    let result = parse_fixture("apple-music-duet.ttml");
    let audio = audio_metadata(&result);

    assert_eq!(audio.duration, Some(216_120));
    assert_eq!(audio.lyric_offset, Some(46));
    assert_eq!(audio.role.as_deref(), Some("spatial"));
    assert_eq!(audio.leading_silence, Some(0));
    assert!(!audio.is_empty());
}

#[test]
fn test_parse_leading_silence_without_audio_element() {
    let result = parse_fixture("line-timing.ttml");
    let audio = audio_metadata(&result);

    assert_eq!(audio.duration, Some(273_476));
    assert_eq!(audio.leading_silence, Some(40));
    assert_eq!(audio.lyric_offset, None);
    assert_eq!(audio.role, None);
}

#[test]
fn test_audio_metadata_absent() {
    let result = parse_ttml(&synthetic_ttml("", "")).unwrap();

    assert!(result.metadata.audio.is_none());
}

#[test]
fn test_invalid_audio_values_are_ignored() {
    let xml = synthetic_ttml(
        r#"<iTunesMetadata leadingSilence="-1">
            <audio lyricOffset="oops" role="  "/>
        </iTunesMetadata>"#,
        r#" dur="unknown""#,
    );

    let result = parse_ttml(&xml).unwrap();

    assert!(result.metadata.audio.is_none());

    // 时间戳溢出同样按缺失处理，而不是让解析器 panic
    let overflow = parse_ttml(&synthetic_ttml("", r#" dur="99999999999999999999""#)).unwrap();
    assert!(overflow.metadata.audio.is_none());
}

#[test]
fn test_first_audio_element_wins() {
    let xml = synthetic_ttml(
        r#"<iTunesMetadata>
            <audio lyricOffset="0.046" role="spatial"/>
            <audio lyricOffset="9.999" role="background"/>
        </iTunesMetadata>"#,
        "",
    );

    let result = parse_ttml(&xml).unwrap();
    let audio = audio_metadata(&result);

    assert_eq!(audio.lyric_offset, Some(46));
    assert_eq!(audio.role.as_deref(), Some("spatial"));
}

#[test]
fn test_generate_audio_metadata() {
    let result = sample_result(Some(AudioMetadata {
        duration: Some(216_120),
        lyric_offset: Some(-271),
        role: Some("spatial".into()),
        leading_silence: Some(40),
    }));

    let generated = generate_ttml(&result, &flat_config()).unwrap();

    assert!(generated.contains(r#"<body dur="3:36.120">"#));
    assert!(generated.contains(
        r#"<iTunesMetadata xmlns="http://music.apple.com/lyric-ttml-internal" leadingSilence="0.040">"#
    ));
    assert!(generated.contains(r#"<audio lyricOffset="-0.271" role="spatial"/>"#));
}

#[test]
fn test_generate_audio_element_with_single_attribute() {
    let role_only = sample_result(Some(AudioMetadata {
        role: Some("spatial".into()),
        ..Default::default()
    }));
    let generated = generate_ttml(&role_only, &flat_config()).unwrap();
    assert!(generated.contains(r#"<audio role="spatial"/>"#));
    assert!(!generated.contains("lyricOffset"));

    let offset_only = sample_result(Some(AudioMetadata {
        lyric_offset: Some(46),
        ..Default::default()
    }));
    let generated = generate_ttml(&offset_only, &flat_config()).unwrap();
    assert!(generated.contains(r#"<audio lyricOffset="0.046"/>"#));
    assert!(!generated.contains(r#"role=""#));
}

#[test]
fn test_generate_leading_silence_zero() {
    let result = sample_result(Some(AudioMetadata {
        leading_silence: Some(0),
        ..Default::default()
    }));

    let generated = generate_ttml(&result, &flat_config()).unwrap();

    assert!(generated.contains(
        r#"<iTunesMetadata xmlns="http://music.apple.com/lyric-ttml-internal" leadingSilence="0">"#
    ));
    assert!(!generated.contains("<audio"));
}

#[test]
fn test_duration_falls_back_to_last_line_end() {
    let result = sample_result(None);

    let generated = generate_ttml(&result, &flat_config()).unwrap();

    assert!(generated.contains(r#"<body dur="2.000">"#));
    assert!(!generated.contains("<iTunesMetadata"));
}

#[test]
fn test_audio_metadata_ignores_generator_switches() {
    let result = sample_result(Some(AudioMetadata {
        leading_silence: Some(40),
        ..Default::default()
    }));

    for (use_apple_format_rules, line_timing) in [(false, false), (true, false), (false, true)] {
        let config = GeneratorConfig {
            use_apple_format_rules,
            line_timing,
            format: false,
        };

        let generated = generate_ttml(&result, &config).unwrap();

        assert!(generated.contains(r#"leadingSilence="0.040""#));
        assert!(generated.contains(r#"<body dur="2.000">"#));
    }
}

#[test]
fn test_roundtrip_preserves_audio_metadata() {
    let parsed = parse_fixture("apple-music-duet.ttml");
    let expected = audio_metadata(&parsed).clone();

    let generated = generate_ttml(&parsed, &flat_config()).unwrap();
    let reparsed = parse_ttml(&generated).unwrap();

    assert_eq!(audio_metadata(&reparsed), &expected);
}

#[test]
fn test_amll_conversion_drops_audio_metadata() {
    let result = parse_fixture("apple-music-duet.ttml");
    let amll = to_amll_lyrics(result, None);

    assert!(amll.metadata.iter().all(|item| !matches!(
        item.key.as_str(),
        "duration" | "lyricOffset" | "role" | "leadingSilence"
    )));
}
