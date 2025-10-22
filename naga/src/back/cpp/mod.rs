/*!
Backend for C++17.

This backend generates C++ code that can be compiled and run on CPU.
It's primarily intended for testing and validation purposes.

# Limitations
- Only supports compute shaders
- No GPU-specific features (textures, samplers)
- Simplified memory model
- Basic expression support
*/

mod conv;
mod keywords;
mod writer;

use alloc::string::String;
use thiserror::Error;

pub use writer::Writer;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Formatting error")]
    Format(#[from] core::fmt::Error),
    #[error("Unsupported feature: {0}")]
    Unsupported(String),
    #[error("Invalid shader stage: {0:?}")]
    InvalidStage(crate::ShaderStage),
}

pub type BackendResult = Result<(), Error>;

#[derive(Clone, Debug, Default)]
pub struct Options {}

/// Write a WGSL module to a C++ string
pub fn write_string(
    module: &crate::Module,
    info: &crate::valid::ModuleInfo,
    options: &Options,
) -> Result<String, Error> {
    let mut output = String::new();
    let mut writer = Writer::new(&mut output, options);
    writer.write(module, info)?;
    Ok(output)
}
