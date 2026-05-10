//! 工具模块

use std::{
    borrow::Cow,
    result::Result as StdResult,
};

use compact_str::CompactString;
use quick_xml::{
    Reader,
    escape::resolve_predefined_entity,
    events::{
        BytesRef,
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
    },
    model::{
        LyricLine,
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
    },
};

/// 读取指定标签内的所有纯文本内容
pub fn read_text_content(
    reader: &mut Reader<&[u8]>,
    context: &ParserContext,
    end_tag: &str,
) -> Result<CompactString> {
    let mut buf = Vec::new();
    let mut result = CompactString::default();
    let mut depth = 1;

    loop {
        match reader.read_event_with_context(&mut buf, context)? {
            Event::Start(ref e) if e.name().is(end_tag) => depth += 1,
            Event::End(ref e) if e.name().is(end_tag) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Event::Text(e) => {
                result.push_str(std::str::from_utf8(e.as_ref())?);
            }
            Event::GeneralRef(reference) => {
                let resolved = resolve_xml_entity(&reference).with_context(reader, context)?;
                result.push_str(&resolved);
            }
            Event::Eof => return Err(ParseErrorKind::UnexpectedEof).with_context(reader, context),
            _ => (),
        }
        buf.clear();
    }
    Ok(result)
}

/// 解析一个 `<span>` 标签，包括逐字音译/翻译和主歌词中的 `<span>` 标签
///
/// ## 示例
/// ```xml
/// <span begin="1:06.534" end="1:06.929" amll:obscene="false" amll:empty-beat="3">prophecies</span>
/// ```
pub fn parse_basic_syllable(
    reader: &mut Reader<&[u8]>,
    context: &ParserContext,
    span_event: &BytesStart,
) -> Result<Syllable> {
    let mut begin_bytes: Option<Cow<[u8]>> = None;
    let mut end_bytes: Option<Cow<[u8]>> = None;
    let mut obscene = None;
    let mut empty_beat = None;

    span_event.for_each_attr(reader, context, tags::SPAN, |attr| {
        match attr.key.as_ref() {
            attrs::b::BEGIN => begin_bytes = Some(attr.value),
            attrs::b::END => end_bytes = Some(attr.value),
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

    let b_bytes = begin_bytes
        .ok_or_else(|| ParseErrorKind::MissingAttribute(CompactString::const_new(attrs::BEGIN)))
        .with_context(reader, context)?;
    let e_bytes = end_bytes
        .ok_or_else(|| ParseErrorKind::MissingAttribute(CompactString::const_new(attrs::END)))
        .with_context(reader, context)?;

    let start_time = parse_timestamp(&b_bytes).with_attr_context(reader, context, attrs::BEGIN)?;
    let end_time = parse_timestamp(&e_bytes).with_attr_context(reader, context, attrs::END)?;

    let text = read_text_content(reader, context, tags::SPAN)?;

    Ok(Syllable {
        text,
        start_time,
        end_time,
        obscene,
        empty_beat,
        ..Default::default()
    })
}

/// 将 XML 中的实体引用转换为真实字符
pub fn resolve_xml_entity(reference: &BytesRef) -> StdResult<String, ParseErrorKind> {
    if let Some(ch) = reference
        .resolve_char_ref()
        .map_err(|_| ParseErrorKind::EntityError(CompactString::const_new("Invalid char ref")))?
    {
        Ok(ch.to_string())
    } else {
        let name_bytes = reference.as_ref();
        let name = std::str::from_utf8(name_bytes).map_err(|_| {
            ParseErrorKind::EntityError(CompactString::const_new("Invalid UTF-8 entity"))
        })?;

        resolve_predefined_entity(name).map_or_else(
            || Err(ParseErrorKind::EntityError(name.into())),
            |value_bytes| Ok(value_bytes.to_string()),
        )
    }
}

/// 从给定文本中移除括号
///
/// 只有在最外侧有左括号和右括号时才移除
pub fn strip_outer_parens(text: &mut CompactString) {
    let trimmed = text.trim();

    let has_left = trimmed.starts_with(['(', '（']);
    let has_right = trimmed.ends_with([')', '）']);

    if has_left && has_right {
        let mut chars = trimmed.chars();
        chars.next();
        chars.next_back();
        *text = chars.as_str().trim().into();
    }
}

/// 从给定逐字歌词音节数组中移除括号
///
/// 只有在第一个和最后一个音节分别在最外侧有左括号和右括号时才移除
pub fn strip_outer_parens_from_words(words: &mut [Syllable]) {
    if words.is_empty() {
        return;
    }

    if words.len() == 1 {
        if let Some(first) = words.first_mut() {
            strip_outer_parens(&mut first.text);
        }
        return;
    }

    let first_has_left = words
        .first()
        .is_some_and(|w| w.text.trim_start().starts_with(['(', '（']));
    let last_has_right = words
        .last()
        .is_some_and(|w| w.text.trim_end().ends_with([')', '）']));

    if first_has_left && last_has_right {
        if let Some(first) = words.first_mut()
            && let Some(idx) = first.text.find(['(', '（'])
        {
            first.text.remove(idx);
        }
        if let Some(last) = words.last_mut()
            && let Some(idx) = last.text.rfind([')', '）'])
        {
            last.text.remove(idx);
        }
    }
}

/// 从给定逐字歌词音节数组构建纯文本
pub fn build_full_text(words: &[Syllable], always_space: bool) -> CompactString {
    let capacity = words.iter().map(|w| w.text.len() + 1).sum();
    let mut full_text = CompactString::with_capacity(capacity);

    for word in words {
        full_text.push_str(&word.text);
        if always_space || word.ends_with_space.unwrap_or_default() {
            full_text.push(' ');
        }
    }

    while full_text.ends_with(' ') {
        full_text.pop();
    }

    full_text
}

pub fn normalize_line_text(text: &mut CompactString) {
    let mut result = CompactString::with_capacity(text.len());
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
    *text = result;
}

/// 规范化给定歌词音节数组的空格
///
/// 会提取前导和尾随空格并分别标记上一个音节和当前音节的 `ends_with_space`
/// 标志，同时从音节文本内移除空格
pub fn normalize_words_spaces(words: &mut [Syllable]) {
    for i in 0..words.len() {
        let text = &words[i].text;
        let original_len = text.len();

        if original_len == 0 {
            continue;
        }

        let trimmed_start = text.trim_start();
        let leading_spaces_len = original_len - trimmed_start.len();

        // 空音节删除内容并标记上一个音节的空格
        // 不删除音节以便使用者可以通过索引匹配主歌词和逐字音译/翻译
        //（如果歌词作者用空的逐字音译/翻译音节表示占位音节）
        if trimmed_start.is_empty() {
            if i > 0 {
                words[i - 1].ends_with_space = Some(true);
            }
            words[i].text.clear();
            continue;
        }

        let trimmed_both = trimmed_start.trim_end();
        let trailing_spaces_len = trimmed_start.len() - trimmed_both.len();

        if leading_spaces_len > 0 && i > 0 {
            words[i - 1].ends_with_space = Some(true);
        }

        if trailing_spaces_len > 0 {
            words[i].ends_with_space = Some(true);
        }

        if trailing_spaces_len > 0 {
            let new_len = original_len - trailing_spaces_len;
            words[i].text.truncate(new_len);
        }
        if leading_spaces_len > 0 {
            let _ = words[i].text.drain(..leading_spaces_len);
        }
    }
}

/// 标记给定歌词行主歌词或背景人声最后一个音节的 `ends_with_space` 标志为 `true`
pub fn mark_last_syllable_space(line: &mut LyricLine, is_bg: bool) {
    if is_bg {
        if let Some(bg) = &mut line.background_vocal
            && let Some(words) = &mut bg.words
        {
            mark_slice_last_space(words);
        }
    } else if let Some(words) = &mut line.words {
        mark_slice_last_space(words);
    }
}

pub const fn mark_slice_last_space(words: &mut [Syllable]) {
    if let Some(last) = words.last_mut() {
        last.ends_with_space = Some(true);
    }
}

/// 该文本是否应该作为空格标记
///
/// 要求不为空且不包含换行符（经过格式化的 TTML）
pub fn is_spacing_text(text: &str) -> bool {
    !text.is_empty() && !text.contains('\n')
}
