use quick_xml::{
    Reader,
    events::{
        BytesStart,
        Event,
    },
    name::QName,
};

use crate::{
    error::{
        OptionExt as _,
        ParseErrorKind,
        Result,
        ResultExt as _,
        TTMLProcessorError,
    },
    parser::{
        state::ParserContext,
        timestamp::parse_timestamp,
    },
};

pub trait QNameExt {
    fn is(&self, tag: &str) -> bool;
}

impl QNameExt for QName<'_> {
    fn is(&self, tag: &str) -> bool {
        self.as_ref() == tag.as_bytes()
    }
}

pub trait BytesStartExt {
    fn get_attr_value(
        &self,
        key: &str,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
    ) -> Result<Option<String>>;

    fn get_required_attr(
        &self,
        key: &str,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
    ) -> Result<String>;

    #[allow(dead_code)]
    fn get_timestamp_attr(
        &self,
        key: &str,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
    ) -> Result<Option<u32>>;

    fn get_required_timestamp_attr(
        &self,
        key: &str,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
    ) -> Result<u32>;
}

impl BytesStartExt for BytesStart<'_> {
    /// 获取指定的属性，未找到则返回 `None`
    ///
    /// 若读取过程中出现 XML 格式错误（AttrError），将注入当前位置和标签栈上下文
    fn get_attr_value(
        &self,
        key: &str,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
    ) -> Result<Option<String>> {
        for attr_result in self.attributes() {
            let attr = attr_result
                .map_err(ParseErrorKind::from)
                .with_attr_context(reader, context, key)?;

            if attr.key.as_ref() == key.as_bytes() {
                let value_str = std::str::from_utf8(&attr.value)
                    .map_err(TTMLProcessorError::Utf8Error)?
                    .to_string();
                return Ok(Some(value_str));
            }
        }
        Ok(None)
    }

    /// 获取指定的必填属性
    ///
    /// 若未找到，返回带 [`ParserContext`] 的 `MissingAttribute` 错误
    fn get_required_attr(
        &self,
        key: &str,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
    ) -> Result<String> {
        self.get_attr_value(key, reader, context)?
            .context_missing_attr(reader, context, key)
    }

    /// 获取并解析时间戳属性
    ///
    /// 若解析失败，返回带 [`ParserContext`]（含属性名 `key`）的 `InvalidTimestamp` 错误
    fn get_timestamp_attr(
        &self,
        key: &str,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
    ) -> Result<Option<u32>> {
        if let Some(val_str) = self.get_attr_value(key, reader, context)? {
            let ms = parse_timestamp(&val_str).with_attr_context(reader, context, key)?;
            Ok(Some(ms))
        } else {
            Ok(None)
        }
    }

    /// 获取并解析必填的时间戳属性
    ///
    /// 若未找到，通过 `get_required_attr` 返回 `MissingAttribute` 错误；
    /// 若解析失败，返回 `InvalidTimestamp` 错误。
    fn get_required_timestamp_attr(
        &self,
        key: &str,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
    ) -> Result<u32> {
        let val_str = self.get_required_attr(key, reader, context)?;
        parse_timestamp(&val_str).with_attr_context(reader, context, key)
    }
}

pub trait ReaderExt<'a> {
    /// 封装底层的 `read_event_into`，在发生 XML 语法错误时自动注入上下文
    fn read_event_with_context(
        &mut self,
        buf: &'a mut Vec<u8>,
        context: &ParserContext,
    ) -> Result<Event<'a>>;
}

impl<'a> ReaderExt<'a> for Reader<&[u8]> {
    fn read_event_with_context(
        &mut self,
        buf: &'a mut Vec<u8>,
        context: &ParserContext,
    ) -> Result<Event<'a>> {
        self.read_event_into(buf)
            .map_err(ParseErrorKind::XmlError)
            .with_context(self, context)
    }
}
