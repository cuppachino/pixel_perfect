//! # ![Raster Font](https://raw.githubusercontent.com/cuppachino/pixel_perfect/refs/heads/main/assets/brand/logo/raster_font.svg)
//!
//! Raster Font is a declarative format for authoring and using image-backed fonts.
//!
//! ## Design
//!
//! General-purpose text engines are designed to solve many problems at once: shaping,
//! font fallback, layout, rasterization, editing, and platform integration across
//! many scripts and font formats. In game development, working with vector fonts can be burdensome.
//! If all you want is an authored set of glyphs, explicit ligatures, and pixel-perfect rendering,
//! adopting standard font formats and text engines can be a tall order. You may find yourself
//! fighting the shaping model of a text engine, or trying to shoehorn your assets into a format
//! that prioritizes other use cases.
//!
//! Raster Font takes a narrower approach. Instead of deriving glyphs from codepoints
//! through a runtime text pipeline, glyph resolution is baked into the font itself. A font
//! declares the exact input sequences it accepts and the glyph each sequence resolves to.
//! Input patterns are precompiled into efficient matching structures that can be used in a variety
//! of game engines and rendering systems.
//!
//! `raster_font` is not a general-purpose text engine, and it does not attempt to provide the
//! shaping and script support of a full typography stack. It is designed for fonts whose glyph set,
//! ligatures, spacing, and visual behavior are authored explicitly ahead of time. In return,
//! "raster fonts" stay simple to define, predictable to resolve, and easy to
//! render with crisp scaling and exact control over glyph mapping.
//!
//! ## Authoring
//!
//! The Raster Font format supports multi-character sequences, unions of sequences bound to
//! shared glyphs, per-glyph overrides, and manual glyph extraction.
//!
//! Creating a pixel-perfect font is a simple as opening your favorite image editor, drawing some
//! glyphs, and writing a simple TOML file to describe your font's layout and behavior.
//!
//! See the [meta documentation](meta) details on the font format and authoring process.
//!
//! ## Resolution
//!
//! A [`RasterFont`] maps input text to valid glyphs using the [`InputResolver`](tree::InputResolver)
//! API. [`valid`] produces an iterator of leftmost-longest matches, silently skipping unmatched input.
//!
//! ```rust
//! use raster_font::{backend::prelude::*, tree::InputResolver};
//!
//! fn resolve<B: Backend<Resources: SpriteSheet>>(font: &RasterFont<B>) {
//!     let glyphs = font.valid("Hello -> world :)").collect::<Vec<_>>();
//!     // use glyphs...
//! }
//! ```
//!
//! ---
//!
//! ## First-party backends
//!
//! The following crates have a stable/familiar release cycle and are considered reasonable for
//! `raster_font` to maintain first-party backends for:
//!
//! | Backend            | Feature      | Description |
//! | :----------------: | :----------: | :---------- |
//! | [Bevy game engine] | `bevy`       | Enables integration with Bevy’s asset system by providing `Asset` implementations for `RasterFont` and the `RasterFontAssetLoaderPlugin`. |
//!
//! ## Example backends
//!
//! The following examples are available in the crate repository.
//!
//! | Example      | Description |
//! | :----------- | :---------- |
//! | `bevy_asset` | Demonstrates using the first-party bevy backend to load raster fonts as assets. |
//! | `macroquad`  | Demonstrates implementing a custom backend for [Macroquad] and loading a font using the [builder API](crate::builder). |
//!
//! Open an issue if you'd like to see an integration example for a specific engine or framework.
//!
//! ---
//!
//! ## Bevy integration
//!
//! Enable the `bevy` feature to use the built-in Bevy backend.
//!
//! ```no_run
//! # #[cfg(feature = "bevy")]
//! use bevy::prelude::*;
//! use raster_font::prelude::*;
//!
//! fn main() {
//! # #[cfg(feature = "bevy")]
//!     App::new()
//!         .add_plugins((DefaultPlugins, RasterFontAssetLoaderPlugin))
//!         .add_systems(Startup, load_font)
//!         .run();
//! }
//!
//! # #[cfg(feature = "bevy")]
//! fn load_font(asset_server: Res<AssetServer>, mut commands: Commands) {
//!     let font: Handle<RasterFont> =
//!         asset_server.load("font.toml");
//!     
//!     commands.insert_resource(MyFont(font));
//! }
//!
//! # #[cfg(feature = "bevy")]
//! fn render_text(
//!     mut commands: Commands,
//!     font: Res<MyFont>,
//!     layouts: Res<Assets<TextureAtlasLayout>>,
//! ) {
//!     let Some(font) = font_assets.get(font) else {
//!         warn!("Font {font:?} not loaded yet");
//!         return;
//!     };
//!     for glyph in font
//!         .upgrade(&*texture_atlas_layout_assets)
//!         .expect("Texture atlas must be loaded if font is loaded")
//!         .valid("Hello, Bevy world!")
//!         .map(Option::unwrap)
//!     {
//!         // Draw glyph using region and offset data...
//!     }
//! }
//! ```
//!
//! ---
//!
//! ## Common modules
//!
//! - Use [`prelude`] for common types and traits for consuming fonts.
//! - Use [`backend`] if implementing your own integration.
//! - Use [`core`] for core types like [`Sequence`], [`Token`], and [`AtlasIndex`].
//!
//!
//! ## Feature flags
//!
//! | Feature             | Description                                             |
//! | :-----------------: | :------------------------------------------------------ |
//! | `bevy`              | Enables Bevy asset loading and integration              |
//! | `font_sequence_map` | Enables direct sequence lookup via `RasterFont::get`    |
//!
//! [`AtlasIndex`]: crate::core::AtlasIndex
//! [`Sequence`]: crate::token::Sequence
//! [`Token`]: crate::token::Token
//! [`RasterFont`]: crate::prelude::RasterFont
//! [`RasterFont::upgrade`]: crate::prelude::RasterFont::upgrade
//! [`SpriteSheet`]: crate::backend::SpriteSheet
//! [`FontResourceProvider`]: crate::backend::FontResourceProvider
//! [`valid`]: crate::tree::InputResolver::valid
//! [`valid_stream`]: crate::tree::InputResolver::valid_stream
//! [Bevy game engine]: https://bevyengine.org/
//! [Macroquad]: https://macroquad.rs/
pub mod backend;
pub mod builder;
pub mod layout;
pub mod meta;
pub mod token;
// pub mod traits;
pub mod tree;

/// Provides `HashMap` and `HashSet` type aliases that switch between `std` and `bevy` collections
/// based on feature flags.
pub mod collections {
    #[cfg(feature = "bevy")]
    pub use bevy_platform::collections::{HashMap, HashSet, hash_map, hash_set};
    #[cfg(not(feature = "bevy"))]
    pub use std::collections::{HashMap, HashSet, hash_map, hash_set};
}

/// Common imports for using Raster Font.
///
/// This module is intended for **font consumers**: code that loads fonts, resolves
/// input into glyphs, and renders or otherwise uses the resulting data.
///
/// In other words, this prelude is for the *use-site* of a font, not for backend
/// authors or low-level integration code.
#[cfg_attr(
    feature = "bevy",
    doc = "
When the `bevy` feature is enabled, [`bevy_backend::prelude`](backend::bevy_backend::prelude) is \
forwarded into this prelude."
)]
///
/// # When to use this
///
/// Import this when writing gameplay, UI, tools, or examples that *consume* a font:
///
/// ```rust,no_run
/// use raster_font::prelude::*;
/// ```
///
/// If you are implementing a custom backend or working with the lower-level builder
/// and backend traits, prefer importing from those modules directly instead of this
/// prelude. See [`backend`], [`core`], and backend-authoring examples in the crate repository.
///
/// [`RasterFont`]: crate::backend::RasterFont
/// [`InputResolver`]: crate::tree::InputResolver
/// [`LigatureTree`]: crate::tree::LigatureTree
pub mod prelude {
    pub use crate::tree::{InputResolver, LigatureTree};

    #[cfg(not(feature = "bevy"))]
    pub use crate::backend::RasterFont;

    #[cfg(feature = "bevy")]
    pub use crate::backend::bevy_backend::prelude::*;
}

pub mod core {
    use bevy_math::{IRect, IVec2, URect, UVec2};
    #[cfg(feature = "bevy")]
    use bevy_reflect::prelude::*;
    use std::{fmt::Debug, ops::Deref};

    pub use crate::{
        layout::OrdTokenLayout,
        meta::FontMeta,
        token::{Sequence, Token, Unique},
        tree::InputResolver,
    };

    /// Unsigned integer representation of a glyph's size.
    pub type UGlyphSize = UVec2;
    /// Unsigned integer representation of a glyph's min and max coordinates in a texture atlas image.
    pub type UGlyphRegion = URect;
    /// Signed integer representation of a glyph's min and max coordinates.
    pub type IGlyphRegion = IRect;
    /// Signed integer representation of a glyph's pixel offset from its default position in the atlas.
    pub type IGlyphOffset = IVec2;

    /// A strongly-typed index into the texture atlas of a [`RasterFont`].
    ///
    /// Corresponds to a slot in the atlas layout's texture region list.
    ///
    /// [`RasterFont`]: crate::prelude::RasterFont
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(
        feature = "bevy",
        derive(Reflect),
        reflect(Clone, Debug, Default, PartialEq, Hash)
    )]
    #[repr(transparent)]
    pub struct AtlasIndex(pub(crate) usize);

    impl Deref for AtlasIndex {
        type Target = usize;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl AsRef<usize> for AtlasIndex {
        fn as_ref(&self) -> &usize {
            &self.0
        }
    }
}
