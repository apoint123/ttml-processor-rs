use serde::Serialize;
use ttml_processor::error::TTMLProcessorError;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsError {
    pub kind: String,

    pub message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_offset: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_stack: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_attribute: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub offending_string: Option<String>,
}

impl From<Box<Self>> for JsError {
    fn from(err: Box<Self>) -> Self {
        *err
    }
}

impl From<TTMLProcessorError> for Box<JsError> {
    fn from(err: TTMLProcessorError) -> Self {
        Self::new(JsError::from(err))
    }
}

impl From<TTMLProcessorError> for JsError {
    fn from(err: TTMLProcessorError) -> Self {
        let (kind, byte_offset, line_id, tag_stack, current_attribute, offending_string) =
            match &err {
                TTMLProcessorError::ParseError { kind, context } => {
                    let kind_str = format!("{kind:?}")
                        .split('(')
                        .next()
                        .unwrap_or("ParseError")
                        .to_string();
                    (
                        kind_str,
                        Some(context.byte_offset),
                        context.line_id.as_ref().map(ToString::to_string),
                        Some(context.tag_stack.iter().map(ToString::to_string).collect()),
                        context.current_attribute.as_ref().map(ToString::to_string),
                        context.offending_string.as_ref().map(ToString::to_string),
                    )
                }
                TTMLProcessorError::IoError(_) => {
                    ("IoError".to_string(), None, None, None, None, None)
                }
                TTMLProcessorError::Utf8Error(_) => {
                    ("Utf8Error".to_string(), None, None, None, None, None)
                }
                TTMLProcessorError::FromUtf8Error(_) => {
                    ("FromUtf8Error".to_string(), None, None, None, None, None)
                }
            };

        Self {
            kind,
            message: err.to_string(),
            byte_offset,
            line_id,
            tag_stack,
            current_attribute,
            offending_string,
        }
    }
}
