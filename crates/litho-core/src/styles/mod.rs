//! Modular lithophane style trait and registry.
//!
//! Each generation mode (`Flat`, `Curved`, `MultiFlat`, and future styles like
//! `ColorFlat`, `Dome`, or `CylinderLamp`) implements [`LithophaneStyle`] and
//! registers in [`StyleRegistry`].

use serde::{Deserialize, Serialize};

use crate::error::LithoError;
use crate::scene::{GeneratedScene, GenerationOptions, ImageInput};
use crate::settings::{LithophaneSettings, StyleId};

pub mod curved;
pub mod flat;
pub mod multi_flat;

/// Public descriptor exposed to UI and CLI to populate style switchers dynamically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleDescriptor {
    pub id: StyleId,
    pub name: &'static str,
    pub tagline: &'static str,
    pub min_images: usize,
    pub max_images: Option<usize>,
}

/// Trait implemented by every modular lithophane geometry generator.
pub trait LithophaneStyle: Send + Sync {
    /// Static metadata describing this style.
    fn descriptor(&self) -> StyleDescriptor;

    /// Validate input image count and style-specific settings prior to meshing.
    fn validate(&self, inputs: &[ImageInput], settings: &LithophaneSettings)
        -> Result<(), LithoError>;

    /// Generate the complete scene (parts, optional merged solid, 2D previews, report).
    fn generate(
        &self,
        inputs: &[ImageInput],
        settings: &LithophaneSettings,
        opts: &GenerationOptions,
        progress: &mut dyn FnMut(&str, f32),
    ) -> Result<GeneratedScene, LithoError>;
}

/// Central registry of all active lithophane styles.
///
/// Adding a new lithophane style (e.g., flat CMY colour lithophanes, domes, or
/// cylindrical multi-image lamps) requires implementing [`LithophaneStyle`] and
/// adding it to `StyleRegistry::new()`.
pub struct StyleRegistry {
    styles: Vec<Box<dyn LithophaneStyle>>,
}

impl Default for StyleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl StyleRegistry {
    /// Construct the default registry containing all built-in styles.
    pub fn new() -> Self {
        Self {
            styles: vec![
                Box::new(flat::FlatStyle),
                Box::new(curved::CurvedStyle),
                Box::new(multi_flat::MultiFlatStyle),
            ],
        }
    }

    /// Return descriptors for all registered styles in display order.
    pub fn descriptors(&self) -> Vec<StyleDescriptor> {
        self.styles.iter().map(|s| s.descriptor()).collect()
    }

    /// Look up a registered style by [`StyleId`].
    pub fn get(&self, id: StyleId) -> Option<&dyn LithophaneStyle> {
        self.styles
            .iter()
            .find(|s| s.descriptor().id == id)
            .map(|s| s.as_ref())
    }
}
