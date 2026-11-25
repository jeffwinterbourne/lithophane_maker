//! `litho-core`: Fast, watertight 3D lithophane geometry, image processing, and export engine.
//!
//! Designed with zero UI dependencies and a modular [`styles::LithophaneStyle`] architecture
//! so future modes (flat CMY colour lithophanes, domes, cylindrical multi-image lamps) plug in cleanly.

pub mod error;
pub mod heightmap;
pub mod image_proc;
pub mod mesh;
pub mod preview;
pub mod scene;
pub mod settings;
pub mod styles;
pub mod supports;

use error::LithoError;
use scene::{GeneratedScene, GenerationOptions, ImageInput};
use settings::{LithophaneSettings, StyleId};
use styles::StyleRegistry;

/// High-level entrypoint to generate any registered lithophane style.
pub fn generate(
    style_id: StyleId,
    inputs: &[ImageInput],
    settings: &LithophaneSettings,
    opts: &GenerationOptions,
    progress: &mut dyn FnMut(&str, f32),
) -> Result<GeneratedScene, LithoError> {
    let registry = StyleRegistry::new();
    let style = registry
        .get(style_id)
        .ok_or_else(|| LithoError::Settings(format!("unregistered style: {}", style_id.as_str())))?;
    style.generate(inputs, settings, opts, progress)
}
