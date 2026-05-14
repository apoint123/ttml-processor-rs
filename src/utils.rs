use crate::model::Syllable;

/// 从给定文本中移除括号
///
/// 只有在最外侧有左括号和右括号时才移除
pub fn strip_outer_parens(text: &mut String) {
    if let Some(stripped) = strip_outer_parens_str(text) {
        *text = stripped.into();
    }
}

fn strip_outer_parens_str(text: &str) -> Option<&str> {
    let trimmed = text.trim();

    let has_left = trimmed.starts_with(['(', '（']);
    let has_right = trimmed.ends_with([')', '）']);

    if has_left && has_right {
        let mut chars = trimmed.chars();
        chars.next();
        chars.next_back();
        Some(chars.as_str().trim())
    } else {
        None
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
        if let Some(first) = words.first_mut()
            && let Some(stripped) = strip_outer_parens_str(&first.text)
        {
            first.text = stripped.into();
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
#[must_use]
pub fn build_full_text(words: &[Syllable], always_space: bool) -> String {
    let capacity = words.iter().map(|w| w.text.len() + 1).sum();
    let mut full_text = String::with_capacity(capacity);

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

/// 规范化逐行歌词的文本，将连续空白折叠为单个空格并去除首尾空格
pub fn normalize_line_text(text: &mut String) {
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
