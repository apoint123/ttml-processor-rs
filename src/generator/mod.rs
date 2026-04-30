mod body;
mod ext;
mod head;
mod span;
mod sub_lyric;
mod utils;

use quick_xml::Writer;

use crate::{
    constants::{
        attrs,
        tags,
        vals,
    },
    error::Result,
    generator::ext::ElementWriterExt as _,
    model::TTMLResult,
};

/// TTML 生成器配置
#[derive(Debug, Clone, Default)]
pub struct GeneratorConfig {
    /// 是否使用 Apple Music 格式规则
    ///
    /// - `true`:
    ///    - 逐行翻译/音译写入到 `<head>` 中
    ///    - 背景人声在主歌词之前开始会先写入
    ///    - 背景人声容器不包含 `begin` 和 `end` 属性
    /// - `false`:
    ///    - 逐行翻译/音译写入为内嵌 `x-translation` / `x-roman`
    ///
    /// 注意：逐字翻译/音译始终写入到 `<head>` 中
    pub use_apple_format_rules: bool,

    /// 是否输出格式化后的 XML 而不是压缩成一行的
    pub format: bool,
}

/// 将解析后的 TTML 结构体生成为 TTML 字符串
///
/// ## Errors
/// 会在 quick-xml 序列化失败或内容不是 UTF-8 编码时返回错误
pub fn generate_ttml(result: &TTMLResult, config: &GeneratorConfig) -> Result<String> {
    let buffer = Vec::with_capacity(102_400);

    let mut writer = if config.format {
        Writer::new_with_indent(buffer, b' ', 4)
    } else {
        Writer::new(buffer)
    };

    // <tt>
    writer
        .create_element(tags::TT)
        .with_attributes([
            (attrs::XMLNS, vals::NS_TTML),
            (attrs::XMLNS_ITUNES, vals::NS_ITUNES),
            (attrs::XMLNS_TTM, vals::NS_TTM),
            (attrs::XMLNS_TTS, vals::NS_TTS),
            (attrs::XMLNS_AMLL, vals::NS_AMLL),
        ])
        .with_attribute_opt((attrs::ITUNES_TIMING, result.metadata.timing_mode.as_deref()))
        .with_attribute_opt((attrs::XML_LANG, result.metadata.language.as_deref()))
        .write_inner_content(|writer| {
            // <head>
            head::write_head(writer, &result.metadata, &result.lines, config)?;

            // <body>
            body::write_body(writer, &result.lines, config)?;
            Ok(())
        })?;

    Ok(String::from_utf8(writer.into_inner())?)
}
