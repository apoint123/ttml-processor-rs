use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::error::JsError;

#[derive(Serialize)]
pub struct WasmResult<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsError>,
}

pub fn wrap_response<T: Serialize, E: Into<JsError>>(result: Result<T, E>) -> JsValue {
    let response: WasmResult<T> = match result {
        Ok(data) => WasmResult {
            success: true,
            data: Some(data),
            error: None,
        },
        Err(e) => WasmResult {
            success: false,
            data: None,
            error: Some(e.into()),
        },
    };
    serde_wasm_bindgen::to_value(&response).unwrap_or_else(|_| {
        js_sys::JSON::parse(
            r#"{"success":false,"error":{"kind":"SerializationError","message":"Failed to serialize response"}}"#
        ).unwrap_or(JsValue::NULL)
    })
}

pub fn parse_js_arg<T: serde::de::DeserializeOwned>(
    val: JsValue,
) -> Result<Option<T>, Box<JsError>> {
    if val.is_undefined() || val.is_null() {
        Ok(None)
    } else {
        serde_wasm_bindgen::from_value(val).map(Some).map_err(|e| {
            Box::new(JsError {
                kind: "ArgumentParseError".to_string(),
                message: format!("Failed to parse JS argument: {e}"),
                byte_offset: None,
                line_id: None,
                tag_stack: None,
                current_attribute: None,
                offending_string: None,
            })
        })
    }
}

pub fn parse_core_struct<T: serde::de::DeserializeOwned>(
    val: JsValue,
    struct_name: &str,
) -> Result<T, Box<JsError>> {
    serde_wasm_bindgen::from_value(val).map_err(|e| {
        Box::new(JsError {
            kind: "StructParseError".to_string(),
            message: format!("Failed to parse {struct_name} from JS value: {e}"),
            byte_offset: None,
            line_id: None,
            tag_stack: None,
            current_attribute: None,
            offending_string: None,
        })
    })
}
