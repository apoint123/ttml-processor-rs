use miette::{
    Diagnostic,
    NamedSource,
    Report,
    SourceSpan,
};
use thiserror::Error;
use ttml_processor::{
    error::{
        ParseErrorKind,
        TTMLProcessorError,
    },
    parse_ttml,
};

#[derive(Error, Debug, Diagnostic)]
#[error("解析 TTML 时出错: {main_reason}")]
#[diagnostic(help("{help_advice}"))]
struct BeautifulTtmlDiagnostic {
    main_reason: String,

    help_advice: String,

    #[source_code]
    src: NamedSource<String>,

    #[label("{label_msg}")]
    err_span: SourceSpan,

    label_msg: String,
}

fn main() -> miette::Result<()> {
    let bad_ttml = r#"<tt xmlns="http://www.w3.org/ns/ttml" xml:lang="zh">
    <head>
        <metadata>
            <ttm:agent type="person" xml:id="v1">
                <ttm:name type="full">Ryan Gosling</ttm:name>
            </ttm:agent>
        </metadata>
    </head>
    <body>
        <div itunes:songPart="Verse">
            <p begin="10.522" end="13.518" itunes:key="L1" ttm:agent="v1">
                <span begin="a" end="13.518">test</span>
            </p>
        </div>
    </body>
</tt>"#;

    if let Err(e) = parse_ttml(bad_ttml) {
        if let TTMLProcessorError::ParseError { kind, context } = e {
            let tag_path = context.tag_stack.join(" > ");
            let line_info = context.line_id.as_deref().unwrap_or("未找到行 ID");
            let attr_info = context.current_attribute.as_deref().unwrap_or("无");
            let offending_info = context.offending_string.as_deref().unwrap_or("无");

            let help_advice = format!(
                "╭─ 错误发生的上下文\n\
                 │ 行 ID    : {line_info}\n\
                 │ 标签路径  : {tag_path}\n\
                 │ 目标属性  : {attr_info}\n\
                 │ 触发内容  : {offending_info}\n\
                 ╰──────────────────────────────"
            );

            let label_msg = match &kind {
                ParseErrorKind::InvalidTimestamp(v) => format!("未能解析时间戳 '{v}'"),
                ParseErrorKind::MissingAttribute(a) => format!("缺少必须的 \"{a}\" 属性"),
                ParseErrorKind::UnexpectedEof => "意外的文件结尾".to_string(),
                ParseErrorKind::XmlError(xml_err) => format!("XML 语法错误: {xml_err}"),
                _ => format!("{kind}"),
            };

            let mut exact_offset = context.byte_offset.saturating_sub(1) as usize;
            let mut exact_len = 1;
            let search_end = usize::try_from(context.byte_offset)
                .ok()
                .map_or(bad_ttml.len(), |v| v.min(bad_ttml.len()));

            if let Some(offending) = context.offending_string.as_deref() {
                if let Some(idx) = bad_ttml[..search_end].rfind(offending) {
                    exact_offset = idx;
                    exact_len = offending.len();
                }
            } else {
                match &kind {
                    ParseErrorKind::MissingAttribute(_a) => {
                        if let Some(tag) = context.tag_stack.last() {
                            let search_str = format!("<{tag}");
                            if let Some(idx) = bad_ttml[..search_end].rfind(&search_str) {
                                exact_offset = idx;
                                exact_len = search_str.len() + 1;
                            }
                        }
                    }
                    ParseErrorKind::XmlError(_) => {
                        exact_offset = context.byte_offset.saturating_sub(1) as usize;
                        exact_len = std::cmp::min(5, bad_ttml.len().saturating_sub(exact_offset));
                    }
                    _ => {}
                }
            }
            let radius = 60;

            let start_idx = bad_ttml[..exact_offset]
                .char_indices()
                .rev()
                .nth(radius)
                .map_or(0, |(i, _)| i);

            let end_idx = bad_ttml[exact_offset + exact_len..]
                .char_indices()
                .nth(radius)
                .map_or(bad_ttml.len(), |(i, _)| exact_offset + exact_len + i);

            let mut display_src = String::new();
            let mut display_offset = exact_offset - start_idx;

            if start_idx > 0 {
                display_src.push_str("... ");
                display_offset += 4;
            }

            display_src.push_str(&bad_ttml[start_idx..end_idx]);

            if end_idx < bad_ttml.len() {
                display_src.push_str(" ...");
            }

            let diag = BeautifulTtmlDiagnostic {
                main_reason: kind.to_string(),
                help_advice,
                src: NamedSource::new("bad.ttml", display_src),
                err_span: SourceSpan::new(display_offset.into(), exact_len),
                label_msg,
            };

            return Err(Report::new(diag));
        }

        return Err(miette::miette!("发生其他错误: {e}"));
    }

    Ok(())
}
