//! AMLL 所使用的较简单的数据结构
//!
//! AMLL 格式不区分翻译/音译内容的来源，因此 TTML 中
//! `<translation>` / `<transliteration>` 的 `automaticallyCreated` 标志
//! 在 TTML 与 AMLL 互转时不会保留。

mod model;
mod to_amll;
mod to_ttml;

pub use model::*;
pub use to_amll::to_amll_lyrics;
pub use to_ttml::to_ttml_result;
