//! # ![Raster Font](https://raw.githubusercontent.com/cuppachino/pixel_perfect/refs/heads/main/assets/brand/logo/raster_font.svg)
//!
//! Data-driven raster fonts for pixel art games.
//!
//! `raster_font` maps input text to regions in a texture atlas using a compact,
//! authorable layout format. It supports multi-character sequences, unions that
//! resolve multiple inputs to the same glyph, per-glyph overrides, and manually
//! extracted glyphs for icons or irregular atlas regions.
//!
//! At runtime, input is resolved using **leftmost-longest matching**, so sequences
//! like `->` naturally take precedence over their prefixes such as `-`.
//!
//! ---
//!
//! ## Overview
//!
//! A raster font in this crate consists of three layers:
//!
//! - **Layout** — an ordered sequence of tokens that define which inputs map to
//!   which glyph slots.
//! - **Metadata** — how those glyph slots are packed into a texture atlas.
//! - **Backend resources** — the runtime representation used for rendering.
//!
//! These are intentionally separated so the same font data can be used across
//! different engines or rendering systems.
//!
//! The crate is organized accordingly:
//!
//! - [`token`] — input sequences and token definitions
//! - [`layout`] — parsing and representing layout strings
//! - [`meta`] — the font asset format (TOML)
//! - [`tree`] — runtime glyph matching
//! - [`builder`] — constructing [`RasterFont`]
//! - [`backend`] — backend abstraction
//! - [`bevy_backend`] — Bevy integration *(feature: `bevy`)*
//!
//! ---
//!
//! ## Layout model
//!
//! The layout language is built around two concepts:
//!
//! - [`Sequence`] — an exact input string (`"a"`, `"->"`, `":)"`)
//! - [`Token`] — one or more sequences that resolve to the same glyph
//!
//! Each token contributes exactly one glyph slot in the atlas.
//!
//! ```text
//! a             # one glyph
//! $(abc)        # one glyph, multi-character sequence
//! $(->|=>)      # one glyph, multiple accepted inputs
//! ```
//!
//! Layouts are parsed left-to-right, and ordering determines how glyphs are
//! associated with atlas regions.
//!
//! ---
//!
//! ## Asset format
//!
//! Fonts are typically defined using a TOML file:
//!
//! ```toml
//! name   = "Example Font"
//! image  = "font.png"
//! layout = "abc$(->|=>)"
//!
//! [pack]
//! size   = [8, 8]
//! region = { min = [0, 0], max = [8, 8] }
//! ```
//!
//! This describes:
//!
//! - the source image
//! - the ordered layout
//! - how glyph regions are extracted
//!
//! See [`meta`] for the full format.
//!
//! ---
//!
//! ## Resolving text
//!
//! A [`RasterFont`] resolves input text into glyphs using the
//! [`InputResolver`](tree::InputResolver) API.
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
//! - Matching is **leftmost-longest**
//! - Unmatched input is skipped
//! - Resolution is efficient and streamable
//!
//! ---
//!
//! ## Backends
//!
//! `RasterFont` is backend-agnostic. This crate separates font data from runtime
//! resources so it can integrate with different engines.
//!
//! A backend defines:
//!
//! - atlas type
//! - image type
//! - resource representation
//! - how resources are constructed
//!
//! There are two common integration patterns:
//!
//! ### Owned resources
//!
//! The font directly owns its render resources.
//!
//! Implement [`SpriteSheet`] and use [`RasterFont`] directly.
//!
//! ### External resources
//!
//! Resources are stored elsewhere (e.g. asset systems).
//!
//! Implement [`FontResourceProvider`] and use [`RasterFont::upgrade`] to obtain
//! a [`RasterFontCtx`](backend::RasterFontCtx) for resolution.
//!
//! ---
//!
//! ## Bevy
//!
//! Enable the `bevy` feature to use the built-in Bevy integration.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use raster_font::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins((DefaultPlugins, RasterFontAssetLoaderPlugin))
//!         .add_systems(Startup, load_font)
//!         .run();
//! }
//!
//! fn load_font(asset_server: Res<AssetServer>) {
//!     let _font: Handle<RasterFont> =
//!         asset_server.load("font.toml");
//! }
//! ```
//!
//! The Bevy backend loads fonts as assets containing:
//!
//! - a texture atlas layout
//! - a font image
//!
//! These are resolved at runtime via Bevy’s asset system.
//!
//! ---
//!
//! ## Getting started
//!
//! - Use [`prelude`] for common imports
//! - See [`meta`] for authoring fonts
//! - Enable `bevy` for Bevy integration
//! - See [`backend`] if implementing your own integration
//!
//! ---
//!
//! ## Feature flags
//!
//! | Feature             | Description                                             |
//! | :-----------------: | :------------------------------------------------------ |
//! | `bevy`              | Enables Bevy asset loading and integration              |
//! | `font_sequence_map` | Enables direct sequence lookup via `RasterFont::get`    |
//!
//! ---
//!
//! [`Sequence`]: crate::token::Sequence
//! [`Token`]: crate::token::Token
//! [`RasterFont`]: crate::prelude::RasterFont
//! [`RasterFont::upgrade`]: crate::prelude::RasterFont::upgrade
//! [`SpriteSheet`]: crate::backend::SpriteSheet
//! [`FontResourceProvider`]: crate::backend::FontResourceProvider
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
///
/// # What it includes
///
/// The prelude re-exports the core types needed to:
///
/// - hold a loaded [`RasterFont`],
/// - resolve input text into glyph matches via [`InputResolver`],
/// - work with the underlying ligature matcher through [`LigatureTree`].
///
/// When the `bevy` feature is enabled, [`bevy_backend::prelude`](backend::bevy_backend::prelude) is
/// forwarded into this prelude.
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
