use std::borrow::Cow;

use crate::{
    model::Syllable,
    utils::build_full_text,
};

pub fn format_timestamp(total_ms: u32) -> String {
    let ms = total_ms % 1000;
    let total_seconds = total_ms / 1000;
    let s = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let m = total_minutes % 60;
    let h = total_minutes / 60;

    if h > 0 {
        format!("{h}:{m:02}:{s:02}.{ms:03}")
    } else if m > 0 {
        format!("{m}:{s:02}.{ms:03}")
    } else {
        format!("{s}.{ms:03}")
    }
}

/// 取出用于逐行输出的纯文本
///
/// `text` 为空时回退到从音节拼接，以兼容只填写了 `words` 的调用方
///
/// `space_joined`：为 `true` 时音节间始终插入空格，一般用于逐字音译
pub fn line_text<'a>(
    text: &'a str,
    words: Option<&[Syllable]>,
    space_joined: bool,
) -> Cow<'a, str> {
    if !text.is_empty() {
        return Cow::Borrowed(text);
    }

    words.map_or(Cow::Borrowed(""), |words| {
        Cow::Owned(build_full_text(words, space_joined))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(1152), "1.152");
        assert_eq!(format_timestamp(46), "0.046");
        assert_eq!(format_timestamp(10254), "10.254");
        assert_eq!(format_timestamp(0), "0.000");
        assert_eq!(format_timestamp(1000), "1.000");
        assert_eq!(format_timestamp(500), "0.500");
        assert_eq!(format_timestamp(10), "0.010");
        assert_eq!(format_timestamp(100), "0.100");

        assert_eq!(format_timestamp(216_120), "3:36.120");
        assert_eq!(format_timestamp(60000), "1:00.000");

        assert_eq!(format_timestamp(3_816_120), "1:03:36.120");
    }
}
