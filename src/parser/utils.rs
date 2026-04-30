//! 工具模块

use std::result::Result as StdResult;

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
    },
};

/// 读取指定标签内的所有纯文本内容
pub fn read_text_content(
    reader: &mut Reader<&[u8]>,
    context: &ParserContext,
    end_tag: &str,
) -> Result<String> {
    let mut buf = Vec::new();
    let mut result = String::new();
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
    let start_time = span_event.get_required_timestamp_attr(attrs::BEGIN, reader, context)?;
    let end_time = span_event.get_required_timestamp_attr(attrs::END, reader, context)?;

    let obscene = span_event
        .get_attr_value(attrs::AMLL_OBSCENE, reader, context)?
        .map(|v| v == vals::TRUE_STR);
    let empty_beat = span_event
        .get_attr_value(attrs::AMLL_EMPTY_BEAT, reader, context)?
        .and_then(|v| v.parse::<u32>().ok());

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
        .map_err(|_| ParseErrorKind::EntityError("Invalid char ref".to_string()))?
    {
        Ok(ch.to_string())
    } else {
        let name_bytes = reference.as_ref();
        let name = std::str::from_utf8(name_bytes)
            .map_err(|_| ParseErrorKind::EntityError("Invalid UTF-8 entity".to_string()))?;

        resolve_predefined_entity(name).map_or_else(
            || Err(ParseErrorKind::EntityError(name.to_owned())),
            |value_bytes| Ok(value_bytes.to_string()),
        )
    }
}

/// 解析给定的时间戳字符串为毫秒
///
/// 严格按照 Apple Music 会使用的时间戳格式来解析
pub fn parse_timestamp(time_str: &str) -> StdResult<u32, ParseErrorKind> {
    let mut parts = time_str.split(':');

    let parse_part = |s: &str| -> StdResult<f64, ParseErrorKind> {
        s.parse::<f64>()
            .map_err(|_| ParseErrorKind::InvalidTimestamp(time_str.to_string()))
    };

    let total_ms = match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(sec), None, None, None) => parse_part(sec)? * 1000.0,

        (Some(min), Some(sec), None, None) => {
            let min_ms = parse_part(min)? * 60_000.0;
            parse_part(sec)?.mul_add(1000.0, min_ms)
        }

        (Some(hour), Some(min), Some(sec), None) => {
            let hour_ms = parse_part(hour)? * 3_600_000.0;

            let hour_min_ms = parse_part(min)?.mul_add(60_000.0, hour_ms);

            parse_part(sec)?.mul_add(1000.0, hour_min_ms)
        }

        _ => return Err(ParseErrorKind::InvalidTimestamp(time_str.to_string())),
    };

    Ok(total_ms.round() as u32)
}

/// 从给定文本中移除括号
///
/// 只有在最外侧有左括号和右括号时才移除
pub fn strip_outer_parens(text: &mut String) {
    let trimmed = text.trim();

    let has_left = trimmed.starts_with(['(', '（']);
    let has_right = trimmed.ends_with([')', '）']);

    if has_left && has_right {
        let mut chars = trimmed.chars();
        chars.next();
        chars.next_back();
        *text = chars.as_str().trim().to_string();
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
pub fn build_full_text(words: &[Syllable]) -> String {
    let capacity = words.iter().map(|w| w.text.len() + 1).sum();
    let mut full_text = String::with_capacity(capacity);

    for word in words {
        full_text.push_str(&word.text);
        if word.ends_with_space.unwrap_or_default() {
            full_text.push(' ');
        }
    }

    while full_text.ends_with(' ') {
        full_text.pop();
    }

    full_text
}

pub fn normalize_line_text(text: &mut String) {
    *text = text.split_whitespace().collect::<Vec<&str>>().join(" ");
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
            words[i].text.drain(..leading_spaces_len);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp() {
        assert_eq!(parse_timestamp("1.152").unwrap(), 1152);
        assert_eq!(parse_timestamp("0.046").unwrap(), 46);
        assert_eq!(parse_timestamp("10.254").unwrap(), 10254);

        assert_eq!(parse_timestamp("3:36.120").unwrap(), 216_120);
        assert_eq!(parse_timestamp("1:00").unwrap(), 60000);

        assert_eq!(parse_timestamp("1:03:36.120").unwrap(), 3_816_120);

        assert!(matches!(
            parse_timestamp("invalid"),
            Err(ParseErrorKind::InvalidTimestamp(_))
        ));
    }
}
