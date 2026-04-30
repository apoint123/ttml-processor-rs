# TTML Processor

A high-performance Rust library for parsing and generating TTML files, specifically tailored for the Apple Music and [AMLL](<https://github.com/amll-dev/applemusic-like-lyrics>) formats.

> [!CAUTION]
> This project is under heavy development and is not yet stable. Expect breaking changes. **Not ready for production use.**

## A Note on Specialization

This library is **not** a general-purpose TTML subtitle parser. It is specifically designed to handle the unique conventions, metadata structures, and extensions (e.g., `itunes:*` attributes, `<iTunesMetadata>`) found in TTML files used by Apple Music. Attempting to use it on generic TTML subtitle files may result in errors or incomplete data.

## Usage

Add `ttml_processor` to your `Cargo.toml`:
```toml
[dependencies]
ttml_processor = "0.1.0" # Replace with the latest version
```

### Parsing Example

```rust
use ttml_processor::parse_ttml;

fn main() {
    let ttml_content = r#"
    <tt xmlns="http://www.w3.org/ns/ttml" itunes:timing="word">
      <body>
        <div>
          <p begin="5.000" end="10.000">
            <span begin="5.100" end="5.500">Hello </span>
            <span begin="5.600" end="6.000">world</span>
          </p>
        </div>
      </body>
    </tt>
    "#;

    let parsed_data = parse_ttml(ttml_content).expect("Failed to parse TTML");

    assert_eq!(parsed_data.lines.len(), 1);
    let first_line = &parsed_data.lines[0];
    assert_eq!(first_line.start_time, 5000);

    let syllables = &first_line
        .words
        .as_ref()
        .expect("Should have syllables in first line");
    assert_eq!(syllables.len(), 2);
    assert_eq!(syllables[0].text, "Hello");
    assert_eq!(syllables[0].start_time, 5100);
    assert_eq!(syllables[0].ends_with_space, Some(true));

    println!("Successfully parsed TTML: {parsed_data:?}");
}
```

### Generation Example

```rust
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
        text: "Hello".to_string(),
        start_time: 5100,
        end_time: 5500,
        ends_with_space: Some(true),
        ..Default::default()
    });
    line.push_word(Syllable {
        text: "world".to_string(),
        start_time: 5600,
        end_time: 6000,
        ..Default::default()
    });

    let result = TTMLResult {
        metadata: TTMLMetadata {
            timing_mode: Some("word".to_string()),
            language: Some("en".to_string()),
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
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
