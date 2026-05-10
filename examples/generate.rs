use ttml_processor::{
    GeneratorConfig,
    generate_ttml,
    model::{
        LyricLine,
        Syllable,
        TTMLMetadata,
        TTMLResult,
    },
};

fn main() {
    let mut line = LyricLine {
        start_time: 5000,
        end_time: 10000,
        ..Default::default()
    };
    line.push_word(Syllable {
        text: "Hello".into(),
        start_time: 5100,
        end_time: 5500,
        ends_with_space: Some(true),
        ..Default::default()
    });
    line.push_word(Syllable {
        text: "world".into(),
        start_time: 5600,
        end_time: 6000,
        ..Default::default()
    });

    let result = TTMLResult {
        metadata: TTMLMetadata {
            timing_mode: Some("word".into()),
            language: Some("en".into()),
            ..Default::default()
        },
        lines: vec![line],
    };

    let config = GeneratorConfig {
        use_apple_format_rules: false,
        format: true,
    };

    let ttml_string = generate_ttml(&result, &config).expect("Failed to generate TTML");
    println!("{ttml_string}");
}
