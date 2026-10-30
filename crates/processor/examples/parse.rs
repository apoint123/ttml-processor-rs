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
