//! Text rendering utilities for pixel art games, built on top of the
//! [raster font](crate::font) format.

#[cfg(any(feature = "bevy_ui", feature = "bevy_2d"))]
use bevy_color::prelude::*;
#[cfg(feature = "bevy")]
use bevy_ecs::prelude::*;
#[cfg(feature = "bevy")]
use bevy_ecs::schedule::SystemSet;
#[cfg(feature = "bevy")]
use bevy_reflect::prelude::*;
use font::backend::Backend;

#[cfg(feature = "bevy_ui")]
use crate::ui::font_scaling::FontScaling;

/// Common utilities for rendering text with [raster fonts](crate::font).
pub mod prelude {
    #[cfg(feature = "bevy")]
    use font::prelude::BevyBackend;

    #[cfg(not(feature = "bevy"))]
    pub use super::RasterText;
    #[cfg(feature = "bevy")]
    pub type RasterText = super::RasterText<BevyBackend>;
}

/// [`SystemSet`] label for raster text systems.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, SystemSet)]
#[cfg(feature = "bevy")]
pub struct RasterTextSystems;

/// Text rendered using a raster font. This component does nothing on its own.
#[cfg_attr(
    all(feature = "composite_text", feature = "per_glyph_text"),
    doc = "Pick a rendering system by inserting [`CompositeText`] or [`PerGlyphText`] alongside `RasterText`."
)]
#[cfg_attr(
    any(feature = "composite_text", feature = "per_glyph_text"),
    doc = "
# Available backends"
)]
#[cfg_attr(
    all(feature = "composite_text"),
    doc = "
## Composite"
)]
#[cfg_attr(
    all(feature = "composite_text", feature = "bevy_ui"),
    doc = "
  - **UI**: Pair with [`CompositeText`] and an [`ImageNode`] to render text inside a UI hierarchy.
    - The `content_size` of the node is automatically updated to match the size of the rendered text
      with respect to the current UI scale, and the node's [`FontScaling`] mode."
)]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect), reflect(Component))]
#[cfg_attr(feature = "bevy_ui", require(FontScaling))]
#[cfg_attr(any(feature = "bevy_ui", "bevy_2d"), require(RasterTextColor))]
pub struct RasterText<B: Backend> {
    /// Handle to the raster font asset.
    pub font: Handle<B::Font>, // todo
    /// Text content. This is stored as a [`Name`] to utilize its efficient change detection.
    #[cfg(feature = "bevy")]
    pub text: Name,
    /// Text content. This is a [`Cow`] to allow both owned and borrowed strings without forcing
    /// allocation in the common case of string literals.
    #[cfg(not(feature = "bevy"))]
    pub text: std::borrow::Cow<'static, str>,
}

impl<B: Backend> AsRef<str> for RasterText<B> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.text.as_ref()
    }
}

impl<B: Backend> AsRef<[u8]> for RasterText<B> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.text.as_bytes()
    }
}

/// A color used to tint [`RasterText`] with when rendering. Requires a compatible text-rendering
/// component (e.g. [`CompositeText`]) to have an effect.
///
/// This is a separate component to allow independent changes from the text content.
#[cfg(any(feature = "bevy_ui", feature = "bevy_2d"))]
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component, Clone, Debug, Default)]
pub struct RasterTextColor(pub Color);

#[cfg(any(feature = "bevy_ui", feature = "bevy_2d"))]
impl From<Color> for RasterTextColor {
    #[inline]
    fn from(color: Color) -> Self {
        Self(color)
    }
}
#[cfg(any(feature = "bevy_ui", feature = "bevy_2d"))]
impl From<&Color> for RasterTextColor {
    #[inline]
    fn from(color: &Color) -> Self {
        Self(*color)
    }
}
#[cfg(any(feature = "bevy_ui", feature = "bevy_2d"))]
impl From<&RasterTextColor> for Color {
    #[inline]
    fn from(color: &RasterTextColor) -> Self {
        color.0
    }
}
#[cfg(any(feature = "bevy_ui", feature = "bevy_2d"))]
impl From<RasterTextColor> for Color {
    #[inline]
    fn from(color: RasterTextColor) -> Self {
        color.0
    }
}
