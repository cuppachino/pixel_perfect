//! Backend-agnostic abstractions for raster font data.
//!
//! A [`RasterFont`] stores glyph bindings and backend-specific resources, but
//! does not know how to draw anything by itself. The split exists so that the same
//! font data can be used with different rendering backends.
//!
//! # Defining a backend
//!
//! To define a backend, implement the following traits:
//!
//! - [`Backend`] — a zero-sized tag struct that names the associated atlas,
//!   image, and resource types for your backend.
//! - [`BackendBuilder`] — converts a [`RawFont`] (the packed glyph sheet
//!   produced by [`FontAtlasBuilder`]) into [`Backend::Resources`].
//! - [`SpriteSheet`] — resolves an [`AtlasIndex`] to [`UTokenProps`]
//!   (unsigned pixel region and signed draw offset).
//!
#![cfg_attr(
    feature = "bevy",
    doc = "See the [Bevy backend](crate::bevy_backend) for a concrete example."
)]
#![cfg_attr(
    not(feature = "bevy"),
    doc = "See the [Bevy backend](https://github.com/cuppachino/pixel_perfect/tree/raster_font-v0.1.0/crates/raster_font/src/bevy_backend) for an example implementation."
)]
//!
//! # Resolving glyphs
//!
//! [`RasterFont`] implements [`AsRef<LigatureTree<AtlasIndex>>`] and
//! [`MapLigature`], which together provide access to the [`InputResolver`] api.
//! That gives you `find`, `valid`, and gang directly on any font where
//! `B::Resources: SpriteSheet`.
//!
//! When the backend's resources live outside the font (e.g. in a Bevy `Assets` collection), call
//! [`RasterFont::upgrade`] with a [`FontResourceProvider`] to obtain a [`RasterFontCtx`] with
//! borrowed access to the resources. The context can be used as an `InputResolver` just like an
//! owned font.
//!
//! Which pattern to use depends entirely on whether your backend owns its resources or borrows
//! them from an external store. Both paths produce identical glyph resolution behaviour.
//!
//! # Feature flags
//!
//!| Feature | Effect |
//!| :-----: | :----- |
//!| `font_sequence_map` | Adds a `HashMap` alongside the ligature tree, enabling O(1) single-sequence lookup with `RasterFont::get`. |
//!
//! [`FontAtlasBuilder`]: crate::builder::FontAtlasBuilder
//! [`InputResolver`]: crate::tree::InputResolver

#[cfg(feature = "bevy")]
use bevy_reflect::prelude::*;

use std::{error::Error, fmt::Debug, marker::PhantomData};

use crate::{
    builder::{RawFont, UTokenProps},
    collections::HashMap,
    core::{AtlasIndex, Sequence},
    tree::{BuildError as LigatureBindingError, LigatureTree, MapLigature},
};

/// Prelude module for backend implementors.
///
/// Re-exports all traits and types required to implement a rendering backend. Prefer importing
/// from here rather than from individual sub-modules so that your backend implementation is
/// insulated from internal reorganizations.
pub mod prelude {
    pub use super::{Backend, BackendBuilder, FontResourceProvider, RasterFont, SpriteSheet};
    pub use crate::{
        builder::{RawFont, UTokenProps},
        core::{AtlasIndex, IGlyphOffset},
        tree::{AsLigatureTree, MapLigature},
    };
}

/// Marker trait that associates platform-specific types with a rendering backend.
///
/// Implement this trait on a zero-sized tag struct to define the type family for your backend.
///
#[cfg_attr(
    feature = "bevy",
    doc = "See the [Bevy backend](crate::bevy_backend) for an example implementation."
)]
#[cfg_attr(
    not(feature = "bevy"),
    doc = "See the [Bevy backend](https://github.com/cuppachino/pixel_perfect/tree/raster_font-v0.1.0/crates/raster_font/src/bevy_backend) for an example implementation."
)]
pub trait Backend {
    type Atlas;
    type Image;
    /// Backend-specific render resources stored inside every [`RasterFont`] for this backend.
    ///
    /// Fonts that own their resources can implement [`SpriteSheet`] directly on this
    /// type.
    ///
    /// Fonts that borrow resources can use it as input for a [`FontResourceProvider`] to produce
    /// a separate resources view type implementing [`SpriteSheet`].
    type Resources;
}

pub trait BackendBuilder {
    /// The backend this builder targets.
    type Backend: Backend;
    /// Error returned if resource construction fails.
    type Error;
    /// Raw image type expected in the [`RawFont`] passed to [`build_resources`](Self::build_resources).
    type Sheet;

    /// Convert raw font data into backend-specific render resources.
    ///
    /// Called automatically by [`FontAtlasBuilder::build`](crate::builder::FontAtlasBuilder::build)
    /// at the end of the builder pipeline. The [`RawFont`] contains the packed glyph sheet image
    /// and [`UTokenProps`] collection produced by the builder. This function must convert those
    /// into the backend's resource format.
    ///
    /// # Errors
    ///
    /// Returns `Err(Self::Error)` if backend-specific resource creation fails. The error surfaces as
    /// [`FontBuilderError::BackendBuilderError`](crate::builder::errors::FontBuilderError::BackendBuilderError)
    /// in the public API of `FontAtlasBuilder::build`.
    fn build_resources(
        self,
        raw_font: RawFont<Self::Sheet>,
    ) -> Result<<Self::Backend as Backend>::Resources, Self::Error>;
}

/// Provides glyph pixel data (region + render offset) for an [`AtlasIndex`].
pub trait SpriteSheet {
    /// Returns `None` if the index falls outside the bounds of the atlas (e.g. the font was built
    /// with a different resource set than the one being queried).
    fn props(&self, index: &AtlasIndex) -> Option<UTokenProps>;
}

/// Borrows backend-specific render resources for a [`RasterFont`] from an external context.
///
/// Some backends (e.g. Bevy) store assets in a world-level\* resource map rather than inline in
/// the font struct. `FontResourceProvider` lets such backends *upgrade* a [`RasterFont`] into
/// a [`RasterFontCtx`] that can perform full glyph resolution, without embedding the world
/// reference permanently inside the font.
///
/// # Lifetimes
///
/// - `'f` — lifetime of the font.
/// - `'r` — lifetime of borrowed resources returned by [`upgrade_font`](Self::upgrade_font).
///
/// # Associated types
/// - **[`Backend`](Self::Backend)**: the backend this provider is compatible with.
///   This must match the backend of the font being upgraded.
///
/// - **[`Output`](Self::Output)**: the type of the resolved resources view returned by
///   [`upgrade_font`](Self::upgrade_font). This is typically a wrapper around borrowed
///   backend assets (e.g. a Bevy `TextureAtlas`), and must implement [`SpriteSheet`] to be
///   usable for glyph resolution in a [`RasterFontCtx`].
pub trait FontResourceProvider {
    type Backend: Backend;
    type Error: Error;
    type Output<'f, 'r>: SpriteSheet
    where
        Self: 'r;

    /// Prepare a borrowed resources view for the from a font's held resources, returning `None` if
    /// preparation fails
    fn upgrade_font<'f, 'r>(
        &'r self,
        res_in: &'f <Self::Backend as Backend>::Resources,
    ) -> Result<Self::Output<'f, 'r>, Self::Error>;
}

/// # Raster Font
///
/// A fully built raster font, ready for glyph resolution and rendering.
///
/// # Input resolution
///
/// Depending on the backend's resource management strategy, a `RasterFont` may not hold all the
/// data it needs to resolve glyphs on its own.
///
/// ## Owned resources
///
/// Raster fonts with owned resources (i.e. `RasterFont<B::Resources: SpriteSheet>`) can
/// use the [`InputResolver`] API to map input sequences to glyphs without any additional setup.
///
/// ```rust
/// use raster_font::{backend::prelude::*, tree::InputResolver};
///
/// fn resolve_owned_resources<B: Backend<Resources: SpriteSheet>>(font: &RasterFont<B>) {
///     // Iterates leftmost-longest glyph matches, skipping unrecognized characters.
///     let _glyphs = font.valid("Hello :)").collect::<Vec<_>>();
/// }
/// ```
///
/// ## Borrowed resources
///
/// Backends with external resources must use [`upgrade`](Self::upgrade) to obtain a
/// [`RasterFontCtx`] with borrowed resources instead. The context functions the same as an owned
/// font for input resolution, but requires a [`FontResourceProvider`] to construct.
///
/// ```rust
/// use raster_font::{backend::prelude::*, tree::InputResolver};
///
/// fn resolve_borrowed_resources<B: Backend, P>(font: &RasterFont<B>, provider: &P)
/// where
///     P: for<'f, 'r> FontResourceProvider<Backend = B>,
/// {
///     let _glyphs = font
///         .upgrade(provider)
///         .unwrap()
///         .valid("Hello :)")
///         .collect::<Vec<_>>();
/// }
/// ```
///
/// # Feature flags
///
///| Feature               | Effect                                                                     |
///| :-------------------: | :-----                                                                     |
///| `bevy`                | Enables `bevy_asset` integrations.                                         |
///| `font_sequence_map`   | Stores a `HashMap` for O(1) single-sequence lookups with `get`.            |
///
/// [`InputResolver`]: crate::tree::InputResolver
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "bevy",
    derive(Reflect),
    reflect(from_reflect = false),
    reflect(where B: Clone + std::fmt::Debug, <B as Backend>::Resources: Clone + std::fmt::Debug),
    reflect(Clone, Debug),
)]
pub struct RasterFont<B: Backend> {
    /// A human-readable name for the font, if provided in the source metadata.
    pub name: Option<String>,
    /// Marker for the backend this font is built for.
    _backend: PhantomData<B>,
    /// Backend-specific resources needed to render this font.
    pub resources: B::Resources,
    /// The line height for this font, derived from the tallest glyph in the atlas.
    #[allow(
        dead_code,
        reason = "conditional compilation makes this appear unused in some contexts"
    )]
    pub(crate) height: u32,
    /// Aho-Corasick automaton for resolving ligatures and multi-character sequences
    /// to their corresponding glyphs. This is the primary resolution path.
    #[cfg_attr(feature = "bevy", reflect(ignore))]
    pub(crate) tree: LigatureTree<AtlasIndex>,

    /// ## Requires `font_sequence_map` feature.
    ///
    /// Enables O(1) access by sequence key at the cost of duplicating the binding data already
    /// held by the ligature tree.
    #[cfg(feature = "font_sequence_map")]
    pub(crate) sequence_map: HashMap<Sequence, AtlasIndex>,
}

/// `AsRef` and [`MapLigature`] together satisfy the [`InputResolver`] blanket impl for RasterFont
/// when B::Resources: SpriteSheet, giving `find`, `valid`, etc. without any extra boilerplate.
///
/// [`InputResolver`]: crate::tree::InputResolver
impl<B: Backend> AsRef<LigatureTree<AtlasIndex>> for RasterFont<B> {
    #[inline]
    fn as_ref(&self) -> &LigatureTree<AtlasIndex> {
        &self.tree
    }
}

/// [`AsRef<LigatureTree<AtlasIndex>>`] and `MapLigature` together satisfy the [`InputResolver`]
/// blanket impl for RasterFont when B::Resources: SpriteSheet, giving `find`, `valid`, etc. without
/// any extra boilerplate.
///
/// [`InputResolver`]: crate::tree::InputResolver
impl<B: Backend<Resources: SpriteSheet>> MapLigature<AtlasIndex> for RasterFont<B> {
    type Output = Option<UTokenProps>;

    fn map_ligature(&self, index: &AtlasIndex) -> Self::Output {
        self.resources.props(index)
    }
}

impl<B: Backend> RasterFont<B> {
    /// Construct a new `RasterFont` from its component parts.
    ///
    /// Builds the internal [`LigatureTree`] from `sequence_map` and returns an error if the
    /// Aho-Corasick automaton cannot be compiled (e.g. due to an excessively large pattern set).
    ///
    /// In practice this is called by [`FontAtlasBuilder::build`]
    /// rather than directly by user code.
    ///
    /// # Errors
    ///
    /// Returns [`LigatureBindingError`] if the Aho-Corasick automaton fails to compile.
    ///
    /// [`FontAtlasBuilder::build`]: crate::builder::FontAtlasBuilder::build
    pub fn new(
        name: Option<String>,
        height: u32,
        resources: B::Resources,
        sequence_map: HashMap<Sequence, AtlasIndex>,
    ) -> Result<Self, LigatureBindingError> {
        #[cfg(feature = "font_sequence_map")]
        let tree = LigatureTree::try_from_bindings(sequence_map.clone())?;
        #[cfg(not(feature = "font_sequence_map"))]
        let tree = LigatureTree::try_from_bindings(sequence_map)?;

        Ok(Self {
            name,
            _backend: PhantomData,
            height,
            resources,
            #[cfg(feature = "font_sequence_map")]
            sequence_map,
            tree,
        })
    }

    /// Returns a reference to the internal [`LigatureTree`].
    ///
    /// See: [`InputResolver`](crate::tree::InputResolver)
    #[inline]
    pub fn tree(&self) -> &LigatureTree<AtlasIndex> {
        &self.tree
    }

    /// Returns the atlas index for a [`Sequence`] or `None` if the sequence is not found.
    ///
    /// Requires the `font_sequence_map` feature. This provides O(1) access to glyphs by sequence
    /// key, but requires additional memory to store the sequence map alongside the ligature tree.
    #[cfg(feature = "font_sequence_map")]
    #[inline]
    pub fn get(&self, sequence: &Sequence) -> Option<&AtlasIndex> {
        self.sequence_map.get(sequence)
    }
}

impl<B: Backend> RasterFont<B> {
    /// Upgrade this font with externally-held render resources, producing a [`RasterFontCtx`].
    ///
    /// Use this when the backend stores resources outside the font struct (e.g. in a Bevy
    /// `Assets<Image>` collection). For backends that store resources directly on the font, this
    /// is unnecessary because the font can resolve glyphs through the `InputResolver` API.
    ///
    /// Returns `Ctx::Error` if the resource provider cannot prepare the font's resources
    /// (e.g. an asset is unloaded, or there was a mismatch between the font and provider resources).
    ///
    /// # Example
    ///
    /// ```
    /// # use raster_font::{backend::prelude::*, tree::InputResolver};
    /// #
    /// fn render<B: Backend>(
    ///     font: &RasterFont<B>,
    ///     provider: &impl FontResourceProvider<Backend = B>,
    /// ) {
    ///     if let Ok(font_ctx) = font.upgrade(provider) {
    ///         // font_ctx implements InputResolver
    ///         font_ctx.valid("Hello world!").for_each(|glyph| {
    ///             // render glyph
    ///         });
    ///     }
    /// }
    /// ```
    pub fn upgrade<'f, 'r, Ctx: FontResourceProvider<Backend = B>>(
        &'f self,
        ctx: &'r Ctx,
    ) -> Result<RasterFontCtx<'f, 'r, Ctx>, Ctx::Error> {
        let resources = ctx.upgrade_font(&self.resources)?;
        Ok(RasterFontCtx {
            font: self,
            resources,
        })
    }
}

/// A [`RasterFont`] paired with externally-borrowed render resources.
///
/// Created by [`RasterFont::upgrade`]. Holds a shared reference to the font and a
/// [`FontResourceProvider::Output`] resolved from an external context (e.g. a Bevy world).
///
/// Because it implements both [`AsRef<LigatureTree<AtlasIndex>>`] and [`MapLigature`], the context
/// has full access to the [`InputResolver`](crate::tree::InputResolver) API.
///
/// # Lifetimes
///
/// - `'f` — lifetime of the borrowed [`RasterFont`].
/// - `'r` — lifetime of the borrowed backend resources.
pub struct RasterFontCtx<'f, 'r, Ctx: FontResourceProvider + 'r> {
    font: &'f RasterFont<Ctx::Backend>,
    resources: Ctx::Output<'f, 'r>,
}

impl<'f, 'r, Ctx: FontResourceProvider> AsRef<LigatureTree<AtlasIndex>>
    for RasterFontCtx<'f, 'r, Ctx>
{
    #[inline]
    fn as_ref(&self) -> &LigatureTree<AtlasIndex> {
        &self.font.tree
    }
}

impl<'f, 'r, Ctx: FontResourceProvider> MapLigature<AtlasIndex> for RasterFontCtx<'f, 'r, Ctx> {
    type Output = Option<UTokenProps>;

    fn map_ligature(&self, index: &AtlasIndex) -> Self::Output {
        self.resources.props(index)
    }
}
