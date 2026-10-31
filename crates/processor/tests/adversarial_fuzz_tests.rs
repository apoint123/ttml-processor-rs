#![allow(clippy::all, clippy::pedantic)]

use ttml_processor::{
    error::{
        ParseErrorKind,
        TTMLProcessorError,
    },
    parse_ttml,
    utils::normalize_line_text,
};

/// 独立黄金参考实现（Oracle），用于差分模糊测试
fn oracle_normalize_line_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut last_was_space = true;

    for c in text.chars() {
        if c.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(c);
            last_was_space = false;
        }
    }

    if result.ends_with(' ') {
        result.pop();
    }
    result
}

// 简单的确定性伪随机生成器 (Xorshift64)
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    const fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xdead_beef } else { seed },
        }
    }

    const fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    const fn next_range(&mut self, min: usize, max: usize) -> usize {
        if min >= max {
            return min;
        }
        min + (self.next_u64() as usize % (max - min))
    }
}

// =========================================================================
// 1. 原地文本规整 (normalize_line_text) 差分模糊测试与极端边缘测试
// =========================================================================

#[test]
fn test_adversarial_normalize_line_text_edge_cases() {
    let test_cases = [
        "",
        " ",
        "  ",
        "          ",
        "\t",
        "\r\n",
        "\n\r \t \n",
        "a\u{000B}b",
        "\u{000B}a\u{000B}\u{000B}b\u{000B}",
        "hello",
        "   hello",
        "hello   ",
        "   hello   ",
        "hello   world",
        "   hello   world   ",
        "  \t hello \r\n world \t  ",
        // Unicode 各种空白符
        "\u{00A0}",                                 // 不间断空格
        "\u{1680}",                                 // 欧甘字母空格
        "\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}", // en/em space
        "\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200A}",
        "\u{2028}\u{2029}", // 行/段分隔符
        "\u{202F}\u{205F}", // 窄空格/数学空格
        "\u{3000}",         // 全角空格 (CJK)
        "\u{3000}\u{3000}你好\u{3000}\u{3000}世界\u{3000}\u{3000}",
        "  \u{3000} \t \r\n 歌词 \u{2003} 文本 \u{00A0} 测试 \u{3000} ",
        // 多字节 UTF-8 字符（包含 4 字节 Emoji 与零宽连接符）
        "🎶🎵🎤  🎧  🎸🎹",
        "   👨‍👩‍👧‍👦   👩‍❤️‍👨   ",
        "   𠮷野家   𩸽   ",
        // 极长连续空格与文本交错
        &(" ".repeat(1000) + "A" + &" ".repeat(1000) + "B" + &" ".repeat(1000)),
        &(" \u{3000} \t ".repeat(500) + "CJK" + &" \u{3000} ".repeat(500)),
    ];

    for case in test_cases {
        let mut actual = case.to_string();
        normalize_line_text(&mut actual);
        let expected = oracle_normalize_line_text(case);
        assert_eq!(
            actual, expected,
            "Mismatch on input: {:?}\nActual:   {:?}\nExpected: {:?}",
            case, actual, expected
        );
        // 确保结果仍然是合法的 UTF-8，并且首尾不包含空格
        assert!(!actual.starts_with(' '), "Should not start with space");
        assert!(!actual.ends_with(' '), "Should not end with space");
    }
}

#[test]
fn test_fuzz_normalize_line_text_differential() {
    let mut rng = SimpleRng::new(42);

    // 字符池：包含普通 ASCII、ASCII 空白、各种 Unicode 空白、CJK 字符、多字节 Emoji
    let char_pool: Vec<char> = vec![
        'a', 'b', 'c', '1', '2', '3', 'x', 'y', 'z', ' ', '\t', '\n', '\r', '\u{000B}', '\u{00A0}',
        '\u{1680}', '\u{2000}', '\u{2003}', '\u{2028}', '\u{2029}', '\u{202F}', '\u{205F}',
        '\u{3000}', '你', '好', '世', '界', 'あ', 'い', 'う', 'え', 'お', '가', '나', '다', '🎤',
        '🎵', '🎶', '🔥', '💖', '✨',
    ];

    for iter in 0..10_000 {
        let len = rng.next_range(0, 80);
        let mut text = String::with_capacity(len * 4);
        for _ in 0..len {
            let idx = rng.next_range(0, char_pool.len());
            text.push(char_pool[idx]);
        }

        let mut actual = text.clone();
        normalize_line_text(&mut actual);
        let expected = oracle_normalize_line_text(&text);

        assert_eq!(
            actual, expected,
            "Fuzz iteration {} failed for text: {:?}",
            iter, text
        );
    }
}

/// U+000B 是 Unicode White_Space，但不在 ASCII 空白集合内，必须与旧实现一样折叠
#[test]
fn test_adversarial_vertical_tab_line_text() {
    let xml = r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><div>
        <p begin="0.0" end="1.0">a&#11;b</p>
    </div></body></tt>"#;

    let parsed = parse_ttml(xml).expect("Should parse a line-level lyric with a char reference");
    assert_eq!(parsed.lines[0].text, "a b");
}

// =========================================================================
// 2. 时间戳格式与边界条件解析测试 (通过 parse_ttml 进行集成验证)
// =========================================================================

#[test]
fn test_adversarial_timestamp_fast_paths_and_boundaries() {
    let cases = [
        ("0:00.000", 0),
        ("3:36.120", 216_120),
        ("9:59.999", 599_999),
        ("00:00.000", 0),
        ("03:36.120", 216_120),
        ("99:59.999", 5_999_999),
        ("00.000", 0),
        ("10.254", 10_254),
        ("99.999", 99_999),
        ("0.000", 0),
        ("1.152", 1_152),
        ("9.999", 9_999),
        ("0", 0),
        ("1", 1_000),
        ("1.1", 1_100),
        ("1.12", 1_120),
        ("1.123456", 1_123),
        ("1:02:03.456", 3_723_456),
        (":10.0", 10_000),
        ("4294967.295", u32::MAX),
    ];

    for (ts_str, expected_ms) in cases {
        let xml = format!(
            r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><div>
                <p begin="{ts_str}" end="{ts_str}">
                    <span begin="{ts_str}" end="{ts_str}">T</span>
                </p>
            </div></body></tt>"#
        );
        let parsed = parse_ttml(&xml)
            .unwrap_or_else(|e| panic!("Failed to parse timestamp {ts_str}: {e:?}"));
        assert_eq!(parsed.lines[0].start_time, expected_ms);
        assert_eq!(parsed.lines[0].end_time, expected_ms);
        assert_eq!(
            parsed.lines[0].words.as_ref().unwrap()[0].start_time,
            expected_ms
        );
        assert_eq!(
            parsed.lines[0].words.as_ref().unwrap()[0].end_time,
            expected_ms
        );
    }

    let invalid_cases = [
        "3:3a.120",
        "a:36.120",
        "3:36.12a",
        "0a:36.120",
        "03:a6.120",
        "1a.254",
        "1.15a",
        "4294967.296",
        "999999999999999999999",
        "1:2:3:4.5",
        "10.:0",
        "10.0.0",
    ];

    for bad_ts in invalid_cases {
        let xml = format!(
            r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><div>
                <p begin="0.0" end="1.0">
                    <span begin="{bad_ts}" end="1.0">T</span>
                </p>
            </div></body></tt>"#
        );
        let err = parse_ttml(&xml);
        assert!(
            err.is_err(),
            "Invalid timestamp {bad_ts} must return an error"
        );
    }
}

// =========================================================================
// 3. 普通音节与背景音节混排、单趟属性提取及空属性对抗测试
// =========================================================================

#[test]
fn test_adversarial_interleaved_syllables_and_bg_vocals() {
    let xml = r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:ttm="http://www.w3.org/ns/ttml#metadata" xmlns:amll="http://www.example.com/amll">
        <body>
            <div>
                <!-- 行 1: 前置普通音节 + 背景人声 + 后置普通音节 -->
                <p begin="1.000" end="10.000" itunes:key="L1">
                    <span begin="1.000" end="2.000">Word1</span>
                    <span begin="2.000" end="3.000">Word2</span>
                    <span ttm:role="x-bg" begin="3.000" end="6.000">
                        <span begin="3.000" end="4.000">(BgWord1</span>
                        <span begin="4.000" end="5.000">BgWord2)</span>
                    </span>
                    <span begin="6.000" end="7.000">Word3</span>
                </p>

                <!-- 行 2: 多个连续背景人声块混排 -->
                <p begin="11.000" end="20.000" itunes:key="L2">
                    <span ttm:role="x-bg">
                        <span begin="11.000" end="12.000">(BgA)</span>
                    </span>
                    <span begin="13.000" end="14.000">Main</span>
                    <span ttm:role="x-bg">
                        <span begin="15.000" end="16.000">(BgB)</span>
                    </span>
                </p>

                <!-- 行 3: 无子音节的纯文本背景人声 (Fallback to raw_text) -->
                <p begin="21.000" end="30.000" itunes:key="L3">
                    <span begin="21.000" end="22.000">Lead</span>
                    <span ttm:role="x-bg" begin="22.500" end="25.000"> (Background humming) </span>
                </p>

                <!-- 行 4: 嵌套/内嵌翻译音译与普通音节混排 -->
                <p begin="31.000" end="40.000" itunes:key="L4">
                    <span begin="31.000" end="32.000">Live</span>
                    <span ttm:role="x-translation" xml:lang="zh-CN">活着</span>
                    <span ttm:role="x-roman" xml:lang="en">huózhe</span>
                    <span begin="32.000" end="33.000">Life</span>
                </p>
            </div>
        </body>
    </tt>"#;

    let parsed = parse_ttml(xml).expect("Should successfully parse interleaved TTML");
    let lines = parsed.lines;
    assert_eq!(lines.len(), 4);

    // 验证行 1
    let line1 = &lines[0];
    let words1 = line1.words.as_ref().unwrap();
    assert_eq!(words1.len(), 3);
    assert_eq!(words1[0].text, "Word1");
    assert_eq!(words1[1].text, "Word2");
    assert_eq!(words1[2].text, "Word3");

    let bg1 = line1.background_vocal.as_ref().unwrap();
    assert_eq!(bg1.start_time, 3000);
    assert_eq!(bg1.end_time, 6000);
    let bg_words1 = bg1.words.as_ref().unwrap();
    assert_eq!(bg_words1.len(), 2);
    assert_eq!(bg_words1[0].text, "BgWord1");
    assert_eq!(bg_words1[1].text, "BgWord2");

    // 验证行 3 (纯文本背景人声)
    let line3 = &lines[2];
    let bg3 = line3.background_vocal.as_ref().unwrap();
    assert_eq!(bg3.start_time, 22500);
    assert_eq!(bg3.end_time, 25000);
    assert!(bg3.words.is_none());
    assert!(bg3.text.contains("Background humming"));

    // 验证行 4 (内嵌翻译/音译)
    let line4 = &lines[3];
    assert_eq!(line4.translations.as_ref().unwrap().len(), 1);
    assert_eq!(line4.translations.as_ref().unwrap()[0].text, "活着");
    assert_eq!(line4.romanizations.as_ref().unwrap().len(), 1);
    assert_eq!(line4.romanizations.as_ref().unwrap()[0].text, "huózhe");
}

#[test]
fn test_adversarial_empty_and_corrupt_attributes() {
    // 缺失 begin 属性 -> 报错 MissingAttribute("begin")
    let missing_begin = r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><div>
        <p begin="1.0" end="2.0">
            <span end="2.0">Word</span>
        </p>
    </div></body></tt>"#;
    let err = parse_ttml(missing_begin).unwrap_err();
    match err {
        TTMLProcessorError::ParseError {
            kind: ParseErrorKind::MissingAttribute(attr),
            ..
        } => {
            assert_eq!(attr, "begin");
        }
        _ => panic!("Expected MissingAttribute('begin'), got: {err:?}"),
    }

    // 缺失 end 属性 -> 报错 MissingAttribute("end")
    let missing_end = r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><div>
        <p begin="1.0" end="2.0">
            <span begin="1.0">Word</span>
        </p>
    </div></body></tt>"#;
    let err = parse_ttml(missing_end).unwrap_err();
    match err {
        TTMLProcessorError::ParseError {
            kind: ParseErrorKind::MissingAttribute(attr),
            ..
        } => {
            assert_eq!(attr, "end");
        }
        _ => panic!("Expected MissingAttribute('end'), got: {err:?}"),
    }

    // 非法时间戳 -> 报错 InvalidTimestamp
    let invalid_ts = r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><div>
        <p begin="1.0" end="2.0">
            <span begin="not_a_time" end="2.0">Word</span>
        </p>
    </div></body></tt>"#;
    let err = parse_ttml(invalid_ts).unwrap_err();
    match err {
        TTMLProcessorError::ParseError {
            kind: ParseErrorKind::InvalidTimestamp(val),
            ..
        } => {
            assert_eq!(val, "not_a_time");
        }
        _ => panic!("Expected InvalidTimestamp, got: {err:?}"),
    }

    // 空属性测试：begin="" 和 end="" 在 fallback 中解析为 0
    let empty_attr = r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><div>
        <p begin="" end="">
            <span begin="" end="" amll:obscene="" amll:empty-beat="">Empty</span>
        </p>
    </div></body></tt>"#;
    let res = parse_ttml(empty_attr).expect("Empty begin/end should fallback to 0ms");
    assert_eq!(res.lines[0].start_time, 0);
    assert_eq!(res.lines[0].end_time, 0);
    let word = &res.lines[0].words.as_ref().unwrap()[0];
    assert_eq!(word.start_time, 0);
    assert_eq!(word.end_time, 0);
    assert_eq!(word.obscene, Some(false));
    assert_eq!(word.empty_beat, None);

    // amll:empty-beat 畸形数值测试（超长数值溢出、负数、非法非数字）
    let strange_amll_attrs = r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:amll="http://www.example.com/amll"><body><div>
        <p begin="1.0" end="2.0">
            <span begin="1.0" end="1.2" amll:obscene="true" amll:empty-beat="999999999999999999999999999">A</span>
            <span begin="1.2" end="1.4" amll:obscene="not_true" amll:empty-beat="-5">B</span>
            <span begin="1.4" end="1.6" amll:obscene="" amll:empty-beat="abc">C</span>
            <span begin="1.6" end="2.0" amll:obscene="false" amll:empty-beat="4">D</span>
        </p>
    </div></body></tt>"#;
    let res = parse_ttml(strange_amll_attrs).expect("Should safely parse strange amll attributes");
    let words = res.lines[0].words.as_ref().unwrap();
    assert_eq!(words[0].obscene, Some(true));
    assert_eq!(words[0].empty_beat, None); // u32 溢出安全转为 None
    assert_eq!(words[1].obscene, Some(false));
    assert_eq!(words[1].empty_beat, None); // 负数解析失败安全转为 None
    assert_eq!(words[2].obscene, Some(false));
    assert_eq!(words[2].empty_beat, None); // 文本解析失败安全转为 None
    assert_eq!(words[3].obscene, Some(false));
    assert_eq!(words[3].empty_beat, Some(4));
}

// =========================================================================
// 4. XML 实体转义字符对抗测试 (&amp;, &quot;, &#x...;)
// =========================================================================

#[test]
fn test_adversarial_xml_entities() {
    let xml = r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:ttm="http://www.w3.org/ns/ttml#metadata">
        <head>
            <metadata>
                <ttm:agent xml:id="agent&amp;1" type="person">
                    <ttm:name type="full">Tom &amp; Jerry &quot;Show&quot;</ttm:name>
                </ttm:agent>
            </metadata>
        </head>
        <body>
            <div>
                <p begin="1.0" end="5.0" itunes:key="L&amp;1" ttm:agent="agent&amp;1">
                    <span begin="1.0" end="2.0">Rock &amp; Roll</span>
                    <span begin="2.0" end="3.0">&lt;Tag&gt;</span>
                    <span begin="3.0" end="4.0">&quot;Quotes&quot; &amp; &apos;Apos&apos;</span>
                    <span begin="4.0" end="5.0">&#x41;&#x42;&#x43; &#65;&#66;&#67;</span>
                </p>
            </div>
        </body>
    </tt>"#;

    let res = parse_ttml(xml).expect("Should parse valid XML entities");
    let line = &res.lines[0];
    assert_eq!(line.id.as_deref(), Some("L&1"));
    assert_eq!(line.agent_id.as_deref(), Some("agent&1"));

    let words = line.words.as_ref().unwrap();
    assert_eq!(words[0].text, "Rock & Roll");
    assert_eq!(words[1].text, "<Tag>");
    assert_eq!(words[2].text, "\"Quotes\" & 'Apos'");
    assert_eq!(words[3].text, "ABC ABC");
}

#[test]
fn test_adversarial_malformed_xml_entities_no_panic() {
    // 包含未定义实体引用 &unknown;
    let malformed_entity = r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><div>
        <p begin="1.0" end="2.0">
            <span begin="1.0" end="2.0">&unknown;</span>
        </p>
    </div></body></tt>"#;
    let err = parse_ttml(malformed_entity);
    assert!(err.is_err(), "Undefined entity must return an error");

    // 属性中包含未闭合实体
    let unclosed_entity = r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><div>
        <p begin="1.0" end="2.0" itunes:key="bad&entity">
            <span begin="1.0" end="2.0">Text</span>
        </p>
    </div></body></tt>"#;
    let _ = parse_ttml(unclosed_entity); // 无论报错还是容错，绝不能 panic
}

// =========================================================================
// 5. 极长文本、超大音节数及深层变异模糊测试 (Fuzzing / Stress Harness)
// =========================================================================

#[test]
fn test_adversarial_huge_line_and_extreme_whitespace() {
    // 构造包含 1,000 个音节的大行，且文本内塞满多字节极端空白符
    let mut huge_xml = String::with_capacity(500_000);
    huge_xml.push_str(r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><div><p begin="0.0" end="1000.0" itunes:key="HUGE">"#);

    for i in 0..1000 {
        let b = format!("{}.000", i);
        let e = format!("{}.800", i);
        let text = format!("  \u{3000} Syl_{} \t \r\n  ", i);
        huge_xml.push_str(&format!(
            r#"<span begin="{}" end="{}">{}</span>"#,
            b, e, text
        ));
    }

    huge_xml.push_str(r#"</p></div></body></tt>"#);

    let res = parse_ttml(&huge_xml).expect("Should handle 1,000 syllables and extreme whitespace");
    assert_eq!(res.lines.len(), 1);
    let words = res.lines[0].words.as_ref().unwrap();
    assert_eq!(words.len(), 1000);
    assert_eq!(words[0].text, "Syl_0");
    assert_eq!(words[999].text, "Syl_999");
}

#[test]
fn test_fuzz_random_byte_mutations_never_panic() {
    // 基于标准合法的 TTML 模板进行随机字节突变模糊测试 (Bit-flip / Byte-replacement / Truncation)
    let base_template = r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:ttm="http://www.w3.org/ns/ttml#metadata" xmlns:amll="http://www.example.com/amll">
        <head>
            <metadata>
                <ttm:agent xml:id="v1" type="person"><ttm:name type="full">Singer</ttm:name></ttm:agent>
            </metadata>
        </head>
        <body>
            <div itunes:songPart="Chorus">
                <p begin="1.234" end="5.678" itunes:key="L1" ttm:agent="v1">
                    <span begin="1.234" end="2.000" amll:obscene="false" amll:empty-beat="2">Hello</span>
                    <span ttm:role="x-bg" begin="2.000" end="4.000">
                        <span begin="2.000" end="3.000">(World)</span>
                    </span>
                    <span begin="4.000" end="5.678">End</span>
                </p>
            </div>
        </body>
    </tt>"#;

    let base_bytes = base_template.as_bytes();
    let mut rng = SimpleRng::new(20261031);

    // 1. 逐步截断测试 (从 0 字节到完整长度)
    for len in 0..base_bytes.len() {
        if let Ok(truncated_str) = std::str::from_utf8(&base_bytes[..len]) {
            let _ = parse_ttml(truncated_str); // 验证任意长度截断下零 panic
        }
    }

    // 2. 随机字节替换与翻转变异测试 (10,000 次)
    let mut mutated = base_bytes.to_vec();
    for _ in 0..10_000 {
        mutated.copy_from_slice(base_bytes);
        let num_mutations = rng.next_range(1, 8);
        for _ in 0..num_mutations {
            let idx = rng.next_range(0, mutated.len());
            let mode = rng.next_range(0, 3);
            match mode {
                0 => mutated[idx] = rng.next_u64() as u8,       // 随机字节
                1 => mutated[idx] ^= 1 << rng.next_range(0, 8), // 单比特翻转
                _ => {
                    mutated[idx] = if rng.next_range(0, 2) == 0 {
                        b'<'
                    } else {
                        b'>'
                    }
                } // 注入标签边界
            }
        }

        if let Ok(mutated_str) = std::str::from_utf8(&mutated) {
            let _ = parse_ttml(mutated_str); // 验证变异输入零 panic、零死循环
        }
    }
}

// =========================================================================
// 6. 复杂 XML 结构对抗：CDATA、XML 注释、深层嵌套及复杂 Unicode 簇
// =========================================================================

#[test]
fn test_adversarial_cdata_and_xml_comments() {
    let xml = r#"<tt xmlns="http://www.w3.org/ns/ttml">
        <!-- 顶部注释 -->
        <body>
            <!-- div 前注释 -->
            <div itunes:songPart="Verse">
                <!-- p 前注释 -->
                <p begin="1.0" end="5.0" itunes:key="L_CDATA">
                    <!-- span 前注释 -->
                    <span begin="1.0" end="2.0"><![CDATA[Raw <Tag> & "Quotes"]]></span>
                    <!-- 两个 span 中间注释 -->
                    <span begin="2.0" end="3.0">Normal</span>
                </p>
                <!-- p 后注释 -->
            </div>
            <!-- div 后注释 -->
        </body>
    </tt>"#;

    let res = parse_ttml(xml).expect("Should successfully parse XML containing CDATA and comments");
    assert_eq!(res.lines.len(), 1);
    let words = res.lines[0].words.as_ref().unwrap();
    // 验证：当前解析器中 Event::CData
    // 属于未定义扩展事件，文本保持为空并在空格规整阶段安全移除，绝不引发 panic
    assert_eq!(words.len(), 1);
    assert_eq!(words[0].text, "Normal");
}

#[test]
fn test_adversarial_deeply_nested_spans() {
    // 构造嵌套 50 层 span 的文本读取测试
    let mut xml = String::from(
        r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><div><p begin="1.0" end="5.0">"#,
    );
    xml.push_str(r#"<span begin="1.0" end="2.0">"#);
    for _ in 0..50 {
        xml.push_str("<span>");
    }
    xml.push_str("DeepText");
    for _ in 0..50 {
        xml.push_str("</span>");
    }
    xml.push_str(r#"</span></p></div></body></tt>"#);

    let res = parse_ttml(&xml).expect("Should parse deeply nested spans without stack overflow");
    let words = res.lines[0].words.as_ref().unwrap();
    assert_eq!(words[0].text, "DeepText");
}

#[test]
fn test_adversarial_complex_unicode_grapheme_clusters() {
    let mut s = "  👍🏽  \u{200B}  👨‍💻  \u{FE0F}  ".to_string();
    normalize_line_text(&mut s);
    // 验证复杂 Unicode 字符簇在原地截断和覆盖规整后保持合法的 UTF-8，不破坏字形码点
    assert!(std::str::from_utf8(s.as_bytes()).is_ok());
    assert!(!s.starts_with(' '));
    assert!(!s.ends_with(' '));
}
