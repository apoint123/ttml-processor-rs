use quick_xml::{
    Writer,
    events::{
        BytesEnd,
        BytesStart,
        Event,
    },
};

use crate::{
    constants::{
        attrs,
        tags,
    },
    error::Result,
    generator::{
        GeneratorConfig,
        ext::ElementWriterExt as _,
        span::write_line_spans,
        sub_lyric::has_inline_sub_lyrics,
        utils::format_timestamp,
    },
    model::LyricLine,
};

/// 写入 `<body>` 部分
pub fn write_body(
    writer: &mut Writer<Vec<u8>>,
    lines: &[LyricLine],
    config: &GeneratorConfig,
) -> Result<()> {
    // 计算歌词时长
    // 理论上这个值应该是歌曲的时长，不过 Apple Music 和其他使用者应该不会在乎这个值的（
    let max_end_time = lines
        .iter()
        .map(|line| {
            let bg_end = line.background_vocal.as_ref().map_or(0, |bg| bg.end_time);
            line.end_time.max(bg_end)
        })
        .max()
        .unwrap_or(0);

    let mut body_start = BytesStart::new(tags::BODY);

    // 添加 dur 属性
    if max_end_time > 0 {
        let dur_str = format_timestamp(max_end_time);
        body_start.push_attribute((attrs::DUR, dur_str.as_str()));
    }

    writer.write_event(Event::Start(body_start))?;

    let all_have_id = lines.iter().all(|line| line.id.is_some());
    let mut line_index = 1;

    for chunk in lines.chunk_by(|a, b| a.block_index.unwrap_or(0) == b.block_index.unwrap_or(0)) {
        write_section(writer, chunk, config, all_have_id, &mut line_index)?;
    }

    writer.write_event(Event::End(BytesEnd::new(tags::BODY)))?;

    Ok(())
}

/// 写入一个 `<div>` 段落
fn write_section(
    writer: &mut Writer<Vec<u8>>,
    chunk: &[LyricLine],
    config: &GeneratorConfig,
    all_have_id: bool,
    line_index: &mut usize,
) -> Result<()> {
    let mut div = writer.create_element(tags::DIV);

    // 计算当前 div 的 begin 和 end 属性
    if let (Some(first), Some(_last)) = (chunk.first(), chunk.last()) {
        let min_start = chunk
            .iter()
            .map(|l| {
                l.background_vocal
                    .as_ref()
                    .map_or(l.start_time, |bg| bg.start_time.min(l.start_time))
            })
            .min()
            .unwrap_or(0);

        let max_end = chunk
            .iter()
            .map(|l| {
                l.background_vocal
                    .as_ref()
                    .map_or(l.end_time, |bg| bg.end_time.max(l.end_time))
            })
            .max()
            .unwrap_or(0);

        if min_start <= max_end {
            let begin_str = format_timestamp(min_start);
            let end_str = format_timestamp(max_end);
            div = div
                .with_attribute((attrs::BEGIN, begin_str.as_str()))
                .with_attribute((attrs::END, end_str.as_str()));
        }

        if let Some(song_part) = first.song_part.as_deref() {
            div = div.with_attribute((attrs::ITUNES_SONGPART, song_part));
        }
    }

    div.write_inner_content(|writer| {
        for line in chunk {
            write_line(writer, line, config, all_have_id, *line_index)?;
            *line_index += 1;
        }
        Ok(())
    })?;

    Ok(())
}

/// 写入一行歌词 `<p>`
fn write_line(
    writer: &mut Writer<Vec<u8>>,
    line: &LyricLine,
    config: &GeneratorConfig,
    all_have_id: bool,
    line_index: usize,
) -> Result<()> {
    let begin = format_timestamp(line.start_time);
    let end = format_timestamp(line.end_time);

    // 允许调用者自定义所有行的 ID，但是如果只自定义了部分行 ID，为了避免 ID
    // 乱序，我们直接忽略残缺行 ID
    let generated_id;
    let itunes_key = if all_have_id {
        line.id.as_deref()
    } else {
        generated_id = format!("L{line_index}");
        Some(generated_id.as_str())
    };

    let p = writer
        .create_element(tags::P)
        .with_attribute((attrs::BEGIN, begin.as_str()))
        .with_attribute((attrs::END, end.as_str()))
        .with_attribute_opt((attrs::ITUNES_KEY, itunes_key))
        .with_attribute_opt((attrs::TTM_AGENT, line.agent_id.as_deref()));

    let should_write_inline_subline = has_inline_sub_lyrics(line, config);

    p.write_inner_content(|writer| {
        write_line_spans(writer, line, should_write_inline_subline, config)?;
        Ok(())
    })?;

    Ok(())
}
