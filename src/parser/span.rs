use compact_str::CompactString;
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
        OptionExt as _,
        ParseErrorKind,
        Result,
        ResultExt as _,
        TTMLProcessorError,
        TTMLResultExt as _,
        TimestampExt as _,
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

    span_event.for_each_attr(reader, context, tags::SPAN, |attr| {
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
        Ok(())
    })?;

    let role_deref = role_bytes.as_deref();
    let is_trans = role_deref == Some(vals::b::ROLE_TRANS);
    let is_rom = role_deref == Some(vals::b::ROLE_ROM);

    // 内嵌翻译 / 音译
    if is_trans || is_rom {
        let lang_offending: CompactString = lang_bytes
            .as_deref()
            .map(|b| String::from_utf8_lossy(b).into_owned().into())
            .unwrap_or_default();
        let lang = lang_bytes
            .map(|v| {
                CompactString::from_utf8(v).map_err(|_| {
                    ParseErrorKind::EntityError(CompactString::const_new("Invalid UTF-8"))
                })
            })
            .transpose()
            .with_context(reader, context)
            .with_offending_string(&lang_offending)?;

        let text = read_text_content(reader, context, tags::SPAN)?;
        let trimmed_text: String = text.trim().into();

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
        let bg_start_str: CompactString = explicit_bg_start_bytes
            .as_deref()
            .map(|b| String::from_utf8_lossy(b).into_owned().into())
            .unwrap_or_default();
        let explicit_bg_start = explicit_bg_start_bytes
            .map(|b| parse_timestamp(b.as_ref()).context_invalid_timestamp(b.as_ref()))
            .transpose()
            .with_attr_context(reader, context, attrs::BEGIN)
            .with_offending_string(&bg_start_str)?;

        let bg_end_str: CompactString = explicit_bg_end_bytes
            .as_deref()
            .map(|b| String::from_utf8_lossy(b).into_owned().into())
            .unwrap_or_default();
        let explicit_bg_end = explicit_bg_end_bytes
            .map(|b| parse_timestamp(b.as_ref()).context_invalid_timestamp(b.as_ref()))
            .transpose()
            .with_attr_context(reader, context, attrs::END)
            .with_offending_string(&bg_end_str)?;

        let mut raw_bg_text = String::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_with_context(&mut buf, context)? {
                Event::Start(ref e) => {
                    let qname = e.name();
                    let tag_name = std::str::from_utf8(qname.as_ref())
                        .map_err(TTMLProcessorError::Utf8Error)?;
                    context.tag_stack.push(tag_name.into());

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
                    raw_bg_text.push_str(
                        &resolve_xml_entity(&reference)
                            .with_context(reader, context)
                            .with_offending_bytes(reference.as_ref())?,
                    );
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
    let mut base_text = CompactString::default();
    let mut ruby_tags = Vec::new();

    let mut obscene = None;
    let mut empty_beat = None;

    // 目前不雅用于和空拍属性暂未明确是添加在何处
    // 暂时直接从容器上提取
    container_event.for_each_attr(reader, context, tags::SPAN, |attr| {
        match attr.key.as_ref() {
            attrs::b::AMLL_OBSCENE => obscene = Some(attr.value.as_ref() == vals::b::TRUE_STR),
            attrs::b::AMLL_EMPTY_BEAT => {
                if let Ok(s) = std::str::from_utf8(attr.value.as_ref()) {
                    empty_beat = s.parse::<u32>().ok();
                }
            }
            _ => {}
        }
        Ok(())
    })?;

    let mut buf = Vec::new();
    loop {
        match reader.read_event_with_context(&mut buf, context)? {
            Event::Start(ref e) => {
                let qname = e.name();
                let tag_name =
                    std::str::from_utf8(qname.as_ref()).map_err(TTMLProcessorError::Utf8Error)?;
                context.tag_stack.push(tag_name.into());

                if qname.is(tags::SPAN) {
                    let ruby_attr = e.get_attr_value(attrs::TTS_RUBY, reader, context)?;
                    match ruby_attr.as_deref() {
                        Some(vals::RUBY_BASE) => {
                            base_text = read_text_content(reader, context, tags::SPAN)?;
                            context.tag_stack.pop();
                        }
                        Some(vals::RUBY_TEXT_CONTAINER) => {
                            ruby_tags = process_ruby_text_container(reader, context)?;
                            context.tag_stack.pop();
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

/// 解析 `tts:ruby="textContainer"` 内部的标签集合
fn process_ruby_text_container(
    reader: &mut Reader<&[u8]>,
    context: &mut ParserContext,
) -> Result<Vec<RubyTag>> {
    let mut ruby_tags = Vec::new();
    let mut inner_buf = Vec::new();

    loop {
        match reader.read_event_with_context(&mut inner_buf, context)? {
            Event::Start(ref inner_e) => {
                let inner_qname = inner_e.name();
                let inner_tag = std::str::from_utf8(inner_qname.as_ref())
                    .map_err(TTMLProcessorError::Utf8Error)?;
                context.tag_stack.push(inner_tag.into());

                if inner_qname.is(tags::SPAN)
                    && let Some(ruby_tag) = process_ruby_text_span(reader, context, inner_e)?
                {
                    ruby_tags.push(ruby_tag);
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
                return Err(ParseErrorKind::UnexpectedEof).with_context(reader, context);
            }
            _ => (),
        }
        inner_buf.clear();
    }

    Ok(ruby_tags)
}

/// 解析单个 `tts:ruby="text"` 标签的内容与属性
fn process_ruby_text_span(
    reader: &mut Reader<&[u8]>,
    context: &ParserContext,
    span_event: &BytesStart,
) -> Result<Option<RubyTag>> {
    let mut is_ruby_text = false;
    let mut r_start = None;
    let mut r_end = None;

    span_event.for_each_attr(reader, context, tags::SPAN, |attr| {
        match attr.key.as_ref() {
            attrs::b::TTS_RUBY => {
                is_ruby_text = attr.value.as_ref() == vals::b::RUBY_TEXT;
            }
            attrs::b::BEGIN => {
                r_start = Some(
                    parse_timestamp(attr.value.as_ref())
                        .context_invalid_timestamp(attr.value.as_ref())
                        .with_attr_context(reader, context, attrs::BEGIN)
                        .with_offending_bytes(&attr.value)?,
                );
            }
            attrs::b::END => {
                r_end = Some(
                    parse_timestamp(attr.value.as_ref())
                        .context_invalid_timestamp(attr.value.as_ref())
                        .with_attr_context(reader, context, attrs::END)
                        .with_offending_bytes(&attr.value)?,
                );
            }
            _ => {}
        }
        Ok(())
    })?;

    if is_ruby_text {
        let start_time = r_start.context_missing_attr(reader, context, attrs::BEGIN)?;
        let end_time = r_end.context_missing_attr(reader, context, attrs::END)?;
        let text = read_text_content(reader, context, tags::SPAN)?;

        Ok(Some(RubyTag {
            text,
            start_time,
            end_time,
        }))
    } else {
        Ok(None)
    }
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
