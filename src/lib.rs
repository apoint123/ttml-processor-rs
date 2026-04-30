mod constants;
pub mod error;
mod generator;
pub mod model;
mod parser;

pub use generator::{
    GeneratorConfig,
    generate_ttml,
};
pub use parser::parse_ttml;
