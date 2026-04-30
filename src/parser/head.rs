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
        Agent,
        PlatformId,
    },
    parser::{
        ext::{
            BytesStartExt as _,
            QNameExt as _,
            ReaderExt as _,
        },
        state::ParserContext,
        sub_lyric::{
            SubLyricType,
            parse_sub_lyrics,
        },
        utils::read_text_content,
    },
};

/// 解析 `<head>` 标签
///
/// ## 示例
/// ```xml
/// <head>
///     <metadata>
///         <ttm:agent type="person" xml:id="v1" />
///         <ttm:agent type="person" xml:id="v2" />
///         <ttm:agent type="person" xml:id="v3" />
///         <ttm:agent type="group" xml:id="v4" />
///         <amll:meta key="title" value="We Don't Talk About Bruno" />
///         <iTunesMetadata xmlns="http://music.apple.com/lyric-ttml-internal">
///             <translations>
///                 <translation type="subtitle" xml:lang="zh-Hans">
///                     翻译内容...
///                 </translation>
///             </translations>
///             <transliterations>
///                 <transliteration>
///                     音译内容...
///                 </transliterations>
///             </transliterations>
///             <songwriters>
///                 <songwriter>Lin-Manuel Miranda</songwriter>
///             </songwriters>
///         </iTunesMetadata>
///     </metadata>
/// </head>
/// ```
pub fn parse_head(reader: &mut Reader<&[u8]>, context: &mut ParserContext) -> Result<()> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_with_context(&mut buf, context)? {
            Event::Start(ref e) => {
                let qname = e.name();
                let name_str =
                    std::str::from_utf8(qname.as_ref()).map_err(TTMLProcessorError::Utf8Error)?;

                context.tag_stack.push(name_str.to_string());

                match name_str {
                    tags::TRANSLATIONS => {
                        parse_sub_lyrics(
                            reader,
                            context,
                            tags::TRANSLATIONS,
                            tags::TRANSLATION,
                            SubLyricType::Translation,
                        )?;
                        context.tag_stack.pop();
                    }
                    tags::TRANSLITERATIONS => {
                        parse_sub_lyrics(
                            reader,
                            context,
                            tags::TRANSLITERATIONS,
                            tags::TRANSLITERATION,
                            SubLyricType::Transliteration,
                        )?;
                        context.tag_stack.pop();
                    }

                    tags::TTM_AGENT => {
                        parse_agent(reader, e, context)?;
                        context.tag_stack.pop();
                    }

                    tags::SONGWRITERS => {
                        parse_songwriters(reader, context)?;
                        context.tag_stack.pop();
                    }

                    tags::TTM_TITLE => {
                        let title = read_text_content(reader, context, tags::TTM_TITLE)?;
                        let trimmed = title.trim();
                        if !trimmed.is_empty() {
                            context.metadata.push_title(trimmed.to_string());
                        }
                        context.tag_stack.pop();
                    }

                    tags::AMLL_META => {
                        if let (Ok(key), Ok(value)) = (
                            e.get_required_attr(attrs::KEY, reader, context),
                            e.get_required_attr(attrs::VALUE, reader, context),
                        ) {
                            parse_amll_meta(context, &key, &value);
                        }
                    }
                    _ => (),
                }
            }
            Event::End(ref e) => {
                if e.name().is(tags::HEAD) {
                    break;
                }
                context.tag_stack.pop();
            }
            Event::Eof => return Err(ParseErrorKind::UnexpectedEof).with_context(reader, context),
            _ => (),
        }
        buf.clear();
    }
    Ok(())
}

/// 解析 `<ttm:agent>` 标签
///
/// ## 示例
/// ```xml
/// <ttm:agent type="person" xml:id="v1" />
/// ```
///
/// 带演唱者名称：
/// ```xml
/// <ttm:agent type="person" xml:id="v1">
///     <ttm:name type="full">Ryan Gosling</ttm:name>
/// </ttm:agent>
/// ```
fn parse_agent(
    reader: &mut Reader<&[u8]>,
    e: &BytesStart,
    context: &mut ParserContext,
) -> Result<()> {
    let id = e.get_required_attr(attrs::XML_ID, reader, context)?;
    let type_ = e.get_attr_value(attrs::TYPE, reader, context)?;
    let mut name = None;

    let mut buf = Vec::new();
    loop {
        match reader.read_event_with_context(&mut buf, context)? {
            Event::Start(ref inner_e) => {
                let qname = inner_e.name();
                let tag_name =
                    std::str::from_utf8(qname.as_ref()).map_err(TTMLProcessorError::Utf8Error)?;

                context.tag_stack.push(tag_name.to_string());

                if inner_e.name().is(tags::TTM_NAME) {
                    name = Some(read_text_content(reader, context, tags::TTM_NAME)?);
                    context.tag_stack.pop();
                }
            }
            Event::End(ref inner_e) => {
                if inner_e.name().is(tags::TTM_AGENT) {
                    break;
                }
                context.tag_stack.pop();
            }
            Event::Eof => return Err(ParseErrorKind::UnexpectedEof).with_context(reader, context),
            _ => (),
        }
        buf.clear();
    }

    let agent = Agent { id, name, type_ };

    context.metadata.insert_agent(agent);

    Ok(())
}

fn parse_amll_meta(context: &mut ParserContext, key: &str, value: &str) {
    let meta = &mut context.metadata;
    let val = value.trim().to_string();
    if val.is_empty() {
        return;
    }

    match key {
        vals::META_MUSIC_NAME => meta.push_title(val),
        vals::META_ARTISTS => meta.push_artist(val),
        vals::META_ALBUM => meta.push_album(val),
        vals::META_ISRC => meta.push_isrc(val),
        vals::META_GITHUB_ID => meta.push_author_id(val),
        vals::META_GITHUB_USER_NAME => meta.push_author_name(val),

        vals::META_NCM_ID | vals::META_QQ_ID | vals::META_SPOTIFY_ID | vals::META_APPLE_ID => {
            let platform = match key {
                vals::META_NCM_ID => PlatformId::NcmMusicId,
                vals::META_QQ_ID => PlatformId::QqMusicId,
                vals::META_SPOTIFY_ID => PlatformId::SpotifyId,
                vals::META_APPLE_ID => PlatformId::AppleMusicId,
                _ => unreachable!(),
            };
            meta.push_platform_id(platform, val);
        }

        _ => {
            meta.push_raw_property(key.to_string(), val);
        }
    }
}

/// 解析 `<songwriters>` 标签
///
/// ## 示例
/// ```xml
/// <songwriters>
///     <songwriter>Lin-Manuel Miranda</songwriter>
/// </songwriters>
/// ```
fn parse_songwriters(reader: &mut Reader<&[u8]>, context: &mut ParserContext) -> Result<()> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_with_context(&mut buf, context)? {
            Event::Start(ref e) => {
                let qname = e.name();
                let tag_name =
                    std::str::from_utf8(qname.as_ref()).map_err(TTMLProcessorError::Utf8Error)?;

                context.tag_stack.push(tag_name.to_string());

                if e.name().is(tags::SONGWRITER) {
                    let text = read_text_content(reader, context, tags::SONGWRITER)?;
                    let text = text.trim().to_string();
                    if !text.is_empty() {
                        context.metadata.push_songwriter(text);
                    }
                    context.tag_stack.pop();
                }
            }
            Event::End(ref e) => {
                if e.name().is(tags::SONGWRITERS) {
                    break;
                }
                context.tag_stack.pop();
            }
            Event::Eof => return Err(ParseErrorKind::UnexpectedEof).with_context(reader, context),
            _ => (),
        }
        buf.clear();
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
    use crate::parser::state::ParserContext;

    fn advance_to_start_tag(reader: &mut Reader<&[u8]>, tag_name: &str) {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().is(tag_name) => break,
                Ok(Event::Eof) => panic!("Reached EOF before finding tag: {tag_name}"),
                _ => (),
            }
            buf.clear();
        }
    }

    #[test]
    fn test_parse_amll_meta() {
        let mut context = ParserContext::default();

        parse_amll_meta(&mut context, vals::META_MUSIC_NAME, "测试歌曲");
        parse_amll_meta(&mut context, vals::META_ARTISTS, "测试歌手");

        parse_amll_meta(&mut context, vals::META_NCM_ID, "12345");
        parse_amll_meta(&mut context, vals::META_APPLE_ID, "67890");

        parse_amll_meta(&mut context, "customKey", "customValue");

        assert_eq!(context.metadata.title.as_ref().unwrap()[0], "测试歌曲");
        assert_eq!(context.metadata.artist.as_ref().unwrap()[0], "测试歌手");

        let platforms = context.metadata.platform_ids.as_ref().unwrap();
        assert_eq!(platforms.get(&PlatformId::NcmMusicId).unwrap()[0], "12345");
        assert_eq!(
            platforms.get(&PlatformId::AppleMusicId).unwrap()[0],
            "67890"
        );

        let raw_props = context.metadata.raw_properties.as_ref().unwrap();
        assert_eq!(raw_props.get("customKey").unwrap()[0], "customValue");
    }

    #[test]
    fn test_parse_songwriters() {
        let xml = r"
        <songwriters>
            <songwriter>Lin-Manuel Miranda</songwriter>
            <songwriter>Germaine Franco</songwriter>
            <songwriter>   </songwriter> 
        </songwriters>";

        let mut reader = Reader::from_str(xml);
        let mut context = ParserContext::default();

        advance_to_start_tag(&mut reader, tags::SONGWRITERS);

        parse_songwriters(&mut reader, &mut context).expect("Failed to parse songwriters");

        let writers = context.metadata.songwriters.as_ref().unwrap();
        assert_eq!(writers.len(), 2);
        assert_eq!(writers[0], "Lin-Manuel Miranda");
        assert_eq!(writers[1], "Germaine Franco");
    }

    #[test]
    fn test_parse_agent() {
        let xml = r#"
        <ttm:agent type="person" xml:id="v1">
            <ttm:name type="full">Ryan Gosling</ttm:name>
        </ttm:agent>"#;

        let mut reader = Reader::from_str(xml);
        let mut context = ParserContext::default();

        let mut buf = Vec::new();
        let start_event = loop {
            match reader.read_event_into(&mut buf).unwrap() {
                Event::Start(e) if e.name().is(tags::TTM_AGENT) => {
                    break e.into_owned();
                }
                _ => (),
            }
        };

        parse_agent(&mut reader, &start_event, &mut context).expect("Failed to parse agent");

        let agents = context.metadata.agents.as_ref().unwrap();
        assert!(agents.contains_key("v1"));

        let agent = agents.get("v1").unwrap();
        assert_eq!(agent.id, "v1");
        assert_eq!(agent.type_.as_deref(), Some("person"));
        assert_eq!(agent.name.as_deref(), Some("Ryan Gosling"));
    }

    #[test]
    fn test_parse_head() {
        let xml = r#"
        <head>
            <metadata>
                <ttm:title>We Don't Talk About Bruno</ttm:title>
                <amll:meta key="album" value="Encanto" />
            </metadata>
        </head>"#;

        let mut reader = Reader::from_str(xml);

        reader.config_mut().expand_empty_elements = true;

        let mut context = ParserContext::default();

        advance_to_start_tag(&mut reader, tags::HEAD);

        parse_head(&mut reader, &mut context).expect("Failed to parse head");

        assert_eq!(
            context.metadata.title.as_ref().unwrap()[0],
            "We Don't Talk About Bruno"
        );
        assert_eq!(context.metadata.album.as_ref().unwrap()[0], "Encanto");
    }
}
