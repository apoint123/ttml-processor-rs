use quick_xml::{
    Reader,
    events::{
        BytesStart,
        Event,
    },
};

use crate::{
    constants::{
        attrs,
        tags,
        vals,
    },
    error::{
        ParseErrorKind,
        Result,
        ResultExt as _,
        TTMLProcessorError,
    },
    model::{
        LyricLine,
        RubyTag,
        SubLyricContent,
        Syllable,
    },
    parser::{
        ext::{
            BytesStartExt as _,
            QNameExt as _,
            ReaderExt as _,
        },
        state::ParserContext,
        timestamp::parse_timestamp,
        utils::{
            is_spacing_text,
            mark_last_syllable_space,
            parse_basic_syllable,
            read_text_content,
            resolve_xml_entity,
        },
    },
};

/// 解析主歌词中的一个 `<span>` 标签
///
/// ## 示例
/// 普通音节样式：
/// ```xml
/// <span begin="1:06.534" end="1:06.929">prophecies</span>
/// ```
///
/// 背景人声样式：
///
/// *添加了整个背景人声区域以便理解*
/// ```xml
/// <span ttm:role="x-bg" begin="21.890" end="24.080">
///     <span begin="21.890" end="22.080">(And </span>
///     <span begin="22.080" end="22.310">there's </span>
///     <span begin="22.310" end="22.410">a </span>
///     <span begin="22.410" end="22.530">lotta </span>
///     <span begin="22.530" end="22.820">cool </span>
///     <span begin="22.820" end="23.160">chicks </span>
///     <span begin="23.160" end="23.530">out </span>
///     <span begin="23.530" end="24.080">there)</span>
/// </span>
/// ```
///
/// 内嵌翻译/音译样式：
/// ```xml
/// <span ttm:role="x-translation" xml:lang="zh-CN">即便不计其数的女孩们对你趋之若鹜</span>
/// ```
pub fn process_span(
    reader: &mut Reader<&[u8]>,
    context: &mut ParserContext,
    span_event: &BytesStart,
    line: &mut LyricLine,
    is_bg_context: bool,
) -> Result<()> {
    let mut role_bytes = None;
    let mut lang_bytes = None;
    let mut is_ruby_container = false;
    let mut explicit_bg_start_bytes = None;
    let mut explicit_bg_end_bytes = None;

    for attr_res in span_event.attributes() {
        let attr = attr_res
            .map_err(ParseErrorKind::from)
            .with_context(reader, context)?;

        match attr.key.as_ref() {
            attrs::b::TTM_ROLE => role_bytes = Some(attr.value),
            attrs::b::XML_LANG => lang_bytes = Some(attr.value),
            attrs::b::TTS_RUBY => {
                is_ruby_container = attr.value.as_ref() == vals::b::RUBY_CONTAINER;
            }
            attrs::b::BEGIN => explicit_bg_start_bytes = Some(attr.value),
            attrs::b::END => explicit_bg_end_bytes = Some(attr.value),
            _ => {}
        }
    }

    let role_deref = role_bytes.as_deref();
    let is_trans = role_deref == Some(vals::b::ROLE_TRANS);
    let is_rom = role_deref == Some(vals::b::ROLE_ROM);

    // 内嵌翻译 / 音译
    if is_trans || is_rom {
        let lang = lang_bytes
            .map(|v| {
                String::from_utf8(v.into_owned())
                    .map_err(|_| ParseErrorKind::EntityError("Invalid UTF-8".to_string()))
            })
            .transpose()
            .with_context(reader, context)?;

        let text = read_text_content(reader, context, tags::SPAN)?;
        let trimmed_text = text.trim().to_string();

        if !trimmed_text.is_empty() {
            let sub_lyric = SubLyricContent {
                language: lang,
                text: trimmed_text,
                words: None, // 内嵌翻译/音译始终只有逐行的
            };

            if is_bg_context {
                let bg = line.bg_vocal_mut();

                if is_trans {
                    bg.push_translation(sub_lyric);
                } else {
                    bg.push_romanization(sub_lyric);
                }
            } else if is_trans {
                line.push_translation(sub_lyric);
            } else {
                line.push_romanization(sub_lyric);
            }
        }
        return Ok(());
    }

    // 背景人声
    let is_bg = is_bg_context || role_deref == Some(b"x-bg");

    if is_bg && !is_bg_context {
        let explicit_bg_start = explicit_bg_start_bytes
            .map(|b| {
                std::str::from_utf8(b.as_ref())
                    .map_err(|_| ParseErrorKind::InvalidTimestamp("Invalid UTF-8".to_string()))
                    .and_then(parse_timestamp)
            })
            .transpose()
            .with_attr_context(reader, context, attrs::BEGIN)?;

        let explicit_bg_end = explicit_bg_end_bytes
            .map(|b| {
                std::str::from_utf8(b.as_ref())
                    .map_err(|_| ParseErrorKind::InvalidTimestamp("Invalid UTF-8".to_string()))
                    .and_then(parse_timestamp)
            })
            .transpose()
            .with_attr_context(reader, context, attrs::END)?;

        let mut raw_bg_text = String::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_with_context(&mut buf, context)? {
                Event::Start(ref e) => {
                    let qname = e.name();
                    let tag_name = std::str::from_utf8(qname.as_ref())
                        .map_err(TTMLProcessorError::Utf8Error)?;
                    context.tag_stack.push(tag_name.to_string());

                    if qname.is(tags::SPAN) {
                        process_span(reader, context, e, line, true)?;
                        context.tag_stack.pop();
                    }
                }
                Event::Text(e) => {
                    let text =
                        std::str::from_utf8(e.as_ref()).map_err(TTMLProcessorError::Utf8Error)?;
                    if is_spacing_text(text) {
                        mark_last_syllable_space(line, is_bg);
                    }
                    raw_bg_text.push_str(text);
                }
                Event::GeneralRef(reference) => {
                    raw_bg_text
                        .push_str(&resolve_xml_entity(&reference).with_context(reader, context)?);
                }
                Event::End(ref e) => {
                    if e.name().is(tags::SPAN) {
                        break;
                    }
                    context.tag_stack.pop();
                }
                Event::Eof => {
                    return Err(ParseErrorKind::UnexpectedEof).with_context(reader, context);
                }
                _ => (),
            }
            buf.clear();
        }

        let bg = line.bg_vocal_mut();

        bg.start_time = explicit_bg_start.unwrap_or_else(|| {
            bg.words
                .as_ref()
                .and_then(|w| w.first())
                .map_or(0, |w| w.start_time)
        });

        bg.end_time = explicit_bg_end.unwrap_or_else(|| {
            bg.words
                .as_ref()
                .and_then(|w| w.last())
                .map_or(0, |w| w.end_time)
        });

        let is_words_empty = bg.words.as_ref().is_none_or(Vec::is_empty);
        if is_words_empty {
            bg.words = None;
            bg.text = raw_bg_text;
        }

        return Ok(());
    }

    // ruby 容器
    if is_ruby_container {
        return process_ruby_container(reader, context, span_event, line, is_bg);
    }

    // 普通音节
    let syllable = parse_basic_syllable(reader, context, span_event)?;

    if is_bg {
        line.bg_vocal_mut().push_word(syllable);
    } else {
        line.push_word(syllable);
    }

    Ok(())
}

/// 解析一个 ruby 容器
///
/// ## 示例
/// ```xml
/// <span tts:ruby="container">
///     <span tts:ruby="base">私</span>
///     <span tts:ruby="textContainer">
///         <span tts:ruby="text" begin="5:08.760" end="5:09.040">わ</span>
///         <span tts:ruby="text" begin="5:09.120" end="5:09.480">た</span>
///         <span tts:ruby="text" begin="5:09.480" end="5:09.950">し</span>
///     </span>
/// </span>
/// ```
fn process_ruby_container(
    reader: &mut Reader<&[u8]>,
    context: &mut ParserContext,
    container_event: &BytesStart,
    line: &mut LyricLine,
    is_bg: bool,
) -> Result<()> {
    let mut base_text = String::new();
    let mut ruby_tags = Vec::new();

    // 目前不雅用于和空拍属性暂未明确是添加在何处
    // 暂时直接从容器上提取
    let obscene = container_event
        .get_attr_value(attrs::AMLL_OBSCENE, reader, context)?
        .map(|v| v == vals::TRUE_STR);
    let empty_beat = container_event
        .get_attr_value(attrs::AMLL_EMPTY_BEAT, reader, context)?
        .and_then(|v| v.parse::<u32>().ok());

    let mut buf = Vec::new();
    loop {
        match reader.read_event_with_context(&mut buf, context)? {
            Event::Start(ref e) => {
                let qname = e.name();
                let tag_name =
                    std::str::from_utf8(qname.as_ref()).map_err(TTMLProcessorError::Utf8Error)?;
                context.tag_stack.push(tag_name.to_string());

                if qname.is(tags::SPAN) {
                    let ruby_attr = e.get_attr_value(attrs::TTS_RUBY, reader, context)?;
                    match ruby_attr.as_deref() {
                        Some(vals::RUBY_BASE) => {
                            base_text = read_text_content(reader, context, tags::SPAN)?;
                            context.tag_stack.pop();
                        }
                        Some(vals::RUBY_TEXT_CONTAINER) => {
                            let mut inner_buf = Vec::new();
                            loop {
                                match reader.read_event_with_context(&mut inner_buf, context)? {
                                    Event::Start(ref inner_e) => {
                                        let inner_qname = inner_e.name();
                                        let inner_tag = std::str::from_utf8(inner_qname.as_ref())
                                            .map_err(TTMLProcessorError::Utf8Error)?;
                                        context.tag_stack.push(inner_tag.to_string());

                                        if inner_qname.is(tags::SPAN)
                                            && inner_e
                                                .get_attr_value(attrs::TTS_RUBY, reader, context)?
                                                .as_deref()
                                                == Some(vals::RUBY_TEXT)
                                        {
                                            let r_start = inner_e.get_required_timestamp_attr(
                                                attrs::BEGIN,
                                                reader,
                                                context,
                                            )?;
                                            let r_end = inner_e.get_required_timestamp_attr(
                                                attrs::END,
                                                reader,
                                                context,
                                            )?;
                                            let r_text =
                                                read_text_content(reader, context, tags::SPAN)?;

                                            ruby_tags.push(RubyTag {
                                                text: r_text,
                                                start_time: r_start,
                                                end_time: r_end,
                                            });
                                            context.tag_stack.pop();
                                        }
                                    }
                                    Event::End(ref inner_e) => {
                                        if inner_e.name().is(tags::SPAN) {
                                            break;
                                        }
                                        context.tag_stack.pop();
                                    }
                                    Event::Eof => {
                                        return Err(ParseErrorKind::UnexpectedEof)
                                            .with_context(reader, context);
                                    }
                                    _ => (),
                                }
                                inner_buf.clear();
                            }
                            context.tag_stack.pop(); // RUBY_TEXT_CONTAINER
                        }
                        _ => {
                            let _ = read_text_content(reader, context, tags::SPAN)?;
                            context.tag_stack.pop();
                        }
                    }
                }
            }
            Event::End(ref e) => {
                if e.name().is(tags::SPAN) {
                    break;
                }
                context.tag_stack.pop();
            }
            Event::Eof => return Err(ParseErrorKind::UnexpectedEof).with_context(reader, context),
            _ => (),
        }
        buf.clear();
    }

    let start_time = ruby_tags.first().map_or(0, |r| r.start_time);
    let end_time = ruby_tags.last().map_or(0, |r| r.end_time);
    let syllable = Syllable {
        text: base_text,
        start_time,
        end_time,
        ruby: Some(ruby_tags),
        obscene,
        empty_beat,
        ..Default::default()
    };

    if is_bg {
        line.bg_vocal_mut().push_word(syllable);
    } else {
        line.push_word(syllable);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use quick_xml::{
        Reader,
        events::Event,
    };

    use super::*;

    fn get_start_event(reader: &mut Reader<&[u8]>) -> BytesStart<'static> {
        let mut buf = Vec::new();
        loop {
            match reader
                .read_event_into(&mut buf)
                .expect("Failed to read XML")
            {
                Event::Start(e) => return e.into_owned(),
                Event::Eof => panic!("Reached EOF before finding start tag"),
                _ => (),
            }
        }
    }

    #[test]
    fn test_process_span_basic_syllable() {
        let xml = r#"<span begin="1:06.534" end="1:06.929">prophecies</span>"#;
        let mut reader = Reader::from_str(xml);
        let mut context = ParserContext::default();

        let start_event = get_start_event(&mut reader);
        let mut line = LyricLine::default();

        process_span(&mut reader, &mut context, &start_event, &mut line, false)
            .expect("Failed to process basic span");

        let words = line.words.as_ref().unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "prophecies");

        assert_eq!(words[0].start_time, 66534);
        assert_eq!(words[0].end_time, 66929);
    }

    #[test]
    fn test_process_span_inline_translation() {
        let xml = r#"<span ttm:role="x-translation" xml:lang="zh-CN">即便不计其数的女孩们对你趋之若鹜</span>"#;
        let mut reader = Reader::from_str(xml);
        let mut context = ParserContext::default();

        let start_event = get_start_event(&mut reader);
        let mut line = LyricLine::default();

        process_span(&mut reader, &mut context, &start_event, &mut line, false)
            .expect("Failed to process translation span");

        assert!(line.words.is_none());

        let translations = line.translations.as_ref().unwrap();
        assert_eq!(translations.len(), 1);
        assert_eq!(translations[0].language.as_deref(), Some("zh-CN"));
        assert_eq!(translations[0].text, "即便不计其数的女孩们对你趋之若鹜");
    }

    #[test]
    fn test_process_span_background_vocal() {
        let xml = r#"
        <span ttm:role="x-bg" begin="21.890" end="24.080">
            <span begin="21.890" end="22.080">(And</span> <span begin="22.080" end="24.080">there's)</span>
        </span>"#;

        let mut reader = Reader::from_str(xml);
        let mut context = ParserContext::default();

        let start_event = get_start_event(&mut reader);
        let mut line = LyricLine::default();

        process_span(&mut reader, &mut context, &start_event, &mut line, false)
            .expect("Failed to process background vocal span");

        assert!(line.words.is_none());

        let bg = line.background_vocal.as_ref().unwrap();
        assert_eq!(bg.start_time, 21890);
        assert_eq!(bg.end_time, 24080);

        let bg_words = bg.words.as_ref().unwrap();
        assert_eq!(bg_words.len(), 2);
        assert_eq!(bg_words[0].text, "(And");
        assert_eq!(bg_words[0].ends_with_space, Some(true));
        assert_eq!(bg_words[1].text, "there's)");
    }

    #[test]
    fn test_process_ruby_container() {
        let xml = r#"
        <span tts:ruby="container" amll:obscene="false">
            <span tts:ruby="base">私</span>
            <span tts:ruby="textContainer">
                <span tts:ruby="text" begin="5:08.760" end="5:09.040">わ</span>
                <span tts:ruby="text" begin="5:09.120" end="5:09.480">た</span>
                <span tts:ruby="text" begin="5:09.480" end="5:09.950">し</span>
            </span>
        </span>"#;

        let mut reader = Reader::from_str(xml);
        let mut context = ParserContext::default();

        let start_event = get_start_event(&mut reader);
        let mut line = LyricLine::default();

        process_span(&mut reader, &mut context, &start_event, &mut line, false)
            .expect("Failed to process ruby container span");

        let words = line.words.as_ref().unwrap();
        assert_eq!(words.len(), 1);

        let word = &words[0];
        assert_eq!(word.text, "私");
        assert_eq!(word.obscene, Some(false));

        assert_eq!(word.start_time, 308_760);
        assert_eq!(word.end_time, 309_950);

        let rubies = word.ruby.as_ref().unwrap();
        assert_eq!(rubies.len(), 3);
        assert_eq!(rubies[0].text, "わ");
        assert_eq!(rubies[0].start_time, 308_760);
        assert_eq!(rubies[2].text, "し");
        assert_eq!(rubies[2].end_time, 309_950);
    }

    #[test]
    fn test_process_span_amll_attributes() {
        let xml = r#"<span begin="1:00" end="1:02" amll:obscene="true" amll:empty-beat="2">badword</span>"#;
        let mut reader = Reader::from_str(xml);
        let mut context = ParserContext::default();

        let start_event = get_start_event(&mut reader);
        let mut line = LyricLine::default();

        process_span(&mut reader, &mut context, &start_event, &mut line, false)
            .expect("Failed to process amll attributes span");

        let words = line.words.as_ref().unwrap();
        assert_eq!(words.len(), 1);

        let word = &words[0];
        assert_eq!(word.text, "badword");

        assert_eq!(word.obscene, Some(true));
        assert_eq!(word.empty_beat, Some(2));
    }

    #[test]
    fn test_process_ruby_container_amll_attributes() {
        let xml = r#"
        <span tts:ruby="container" amll:obscene="true" amll:empty-beat="2">
            <span tts:ruby="base">XX</span>
            <span tts:ruby="textContainer">
                <span tts:ruby="text" begin="0.000" end="1.000">x</span>
                <span tts:ruby="text" begin="1.000" end="2.000">x</span>
            </span>
        </span>"#;

        let mut reader = Reader::from_str(xml);
        let mut context = ParserContext::default();

        let start_event = get_start_event(&mut reader);
        let mut line = LyricLine::default();

        process_span(&mut reader, &mut context, &start_event, &mut line, false)
            .expect("Failed to process ruby container with amll attributes");

        let words = line.words.as_ref().unwrap();
        assert_eq!(words.len(), 1);

        let word = &words[0];
        assert_eq!(word.text, "XX");

        assert_eq!(word.obscene, Some(true));
        assert_eq!(word.empty_beat, Some(2));
    }
}
