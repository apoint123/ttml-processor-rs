#[cfg(feature = "amll")]
use ttml_processor::amll::{
    AmllLyricResult,
    AmllToTtmlOptions,
    TtmlToAmllOptions,
    to_amll_lyrics,
    to_ttml_result,
};
use ttml_processor::{
    GeneratorConfig,
    generate_ttml as core_generate,
    model::TTMLResult,
    parse_ttml as core_parse,
};
use wasm_bindgen::prelude::*;

use crate::{
    error::JsError,
    model::{
        parse_core_struct,
        parse_js_arg,
        wrap_response,
    },
};

/// 解析 TTML 字符串为 `TTMLResult`
#[wasm_bindgen(js_name = parseTtml)]
pub fn parse_ttml(ttml_content: &str) -> JsValue {
    let result = core_parse(ttml_content);
    wrap_response(result)
}

/// 将解析后的 TTML 结构体生成为 TTML 字符串
#[wasm_bindgen(js_name = generateTtml)]
pub fn generate_ttml(ttml_result_val: JsValue, config_val: JsValue) -> JsValue {
    let result: Result<_, Box<JsError>> = (|| {
        let ttml_result = parse_core_struct::<TTMLResult>(ttml_result_val, "TTMLResult")?;
        let config = parse_js_arg::<GeneratorConfig>(config_val)?.unwrap_or_default();
        Ok(core_generate(&ttml_result, &config)?)
    })();

    wrap_response(result)
}

/// 便捷方法，将 TTML 字符串转换并降级为 AMLL 所使用的较简单的结构
#[cfg(feature = "amll")]
#[wasm_bindgen(js_name = ttmlToAmll)]
pub fn ttml_to_amll(ttml_content: &str, options_val: JsValue) -> JsValue {
    let result: Result<_, Box<JsError>> = (|| {
        let options = parse_js_arg::<TtmlToAmllOptions>(options_val)?;
        let ttml_result = core_parse(ttml_content)?;
        Ok(to_amll_lyrics(ttml_result, options.as_ref()))
    })();

    wrap_response(result)
}

/// 便捷方法，将 AMLL 格式的歌词和元数据生成为 TTML 字符串
///
/// 会对文本进行规范化，例如清理空格、移除背景人声括号等
#[cfg(feature = "amll")]
#[wasm_bindgen(js_name = amllToTtml)]
pub fn amll_to_ttml(amll_val: JsValue, options_val: JsValue, config_val: JsValue) -> JsValue {
    let result: Result<_, Box<JsError>> = (|| {
        let options = parse_js_arg::<AmllToTtmlOptions>(options_val)?;
        let config = parse_js_arg::<GeneratorConfig>(config_val)?.unwrap_or_default();
        let amll_result = parse_core_struct::<AmllLyricResult>(amll_val, "AmllLyricResult")?;

        let ttml_result = to_ttml_result(amll_result, options.as_ref());
        Ok(core_generate(&ttml_result, &config)?)
    })();

    wrap_response(result)
}

/// 工具方法，将本解析器复杂的数据结构降级为 AMLL 所使用的较简单的数据结构
#[cfg(feature = "amll")]
#[wasm_bindgen(js_name = ttmlResultToAmll)]
pub fn ttml_result_to_amll(ttml_result_val: JsValue, options_val: JsValue) -> JsValue {
    let result: Result<_, Box<JsError>> = (|| {
        let ttml_result = parse_core_struct::<TTMLResult>(ttml_result_val, "TTMLResult")?;
        let options = parse_js_arg::<TtmlToAmllOptions>(options_val)?;
        Ok(to_amll_lyrics(ttml_result, options.as_ref()))
    })();

    wrap_response(result)
}

/// 工具方法，将 AMLL 格式的歌词和元数据转换为 `TTMLResult` 结构
///
/// 会对文本进行规范化，例如清理空格、移除背景人声括号等
#[cfg(feature = "amll")]
#[wasm_bindgen(js_name = amllToTtmlResult)]
pub fn amll_to_ttml_result(amll_val: JsValue, options_val: JsValue) -> JsValue {
    let result: Result<_, Box<JsError>> = (|| {
        let amll_result = parse_core_struct::<AmllLyricResult>(amll_val, "AmllLyricResult")?;
        let options = parse_js_arg::<AmllToTtmlOptions>(options_val)?;
        Ok(to_ttml_result(amll_result, options.as_ref()))
    })();

    wrap_response(result)
}
