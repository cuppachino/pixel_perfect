//! Builder pattern for constructing a [`RasterFont`] in multiple stages.
//!
//! [`FontAtlasBuilder`] uses Rust's type system to enforce that required build steps are
//! completed in order before the font can be finalized.
//!
//! # Typical build sequence
//!
//! ```rust,no_run
//! # use std::error::Error;
//! # use raster_font::{
//! #     backend::prelude::*,
//! #     builder::{GlyphSheet, FontAtlasBuilder, errors::FontBuilderError},
//! #     meta::FontLayout
//! # };
//! #
//! fn build_font<B: Backend, Builder, Image>(
//!     name: Option<String>,
//!     layout: FontLayout,
//!     sheet: GlyphSheet<Image>,
//!     backend_builder: Builder,
//! ) -> Result<RasterFont<B>, FontBuilderError<Builder::Error>>
//!     where Builder: BackendBuilder<Backend = B, Error: Error, Sheet = Image>
//! {
//!     FontAtlasBuilder::from(layout.unique()) // reserve capacity for the number of unique glyphs and sequences in the layout.
//!         .with_image(sheet)                  // attach the glyph sprite sheet
//!         .populate_layout(&layout)           // compute glyph regions from the layout
//!         .custom_glyphs(layout.custom())?    // (optional) register hand-specified glyphs
//!         .with_name(name)                    // (optional) name the font
//!         .build(backend_builder)             // finalize and upload to the backend
//! }
//! ```
//!
//! Steps that have not yet been completed are simply absent from the type's API -- the compiler
//! rejects any attempt to call `build` before `populate_layout`, for example.
use crate::{
    backend::{Backend, BackendBuilder, RasterFont},
    core::{
        AtlasIndex, IGlyphOffset, IGlyphRegion, Sequence, Token, UGlyphRegion, UGlyphSize, Unique,
    },
    meta::{CustomGlyph, FontLayout, FontTrack, GlyphOverride, PackingMode},
};

pub mod prelude {
    pub use super::{FontAtlasBuilder, GlyphSheet, errors::FontBuilderError};
}

#[cfg(feature = "bevy")]
use bevy_platform::collections::HashMap;
#[cfg(not(feature = "bevy"))]
use std::collections::HashMap;

use std::{error::Error, marker::PhantomData};

/// Glyph rendering properties expressed in *unsigned* pixel coordinates.
#[derive(Clone, Debug)]
pub struct UTokenProps {
    /// The axis-aligned bounding rectangle of the glyph within the texture atlas.
    pub region: UGlyphRegion,
    /// A signed pixel offset applied to the glyph's draw position at render time.
    pub offset: IGlyphOffset,
}

/// Glyph rendering properties in *signed* pixel coordinates, before clamping to the atlas image.
///
/// Used internally during layout population. Coordinates may extend beyond `[0, image_size)`
/// before being clamped to a valid unsigned rectangle for the atlas. Converted to [`UTokenProps`]
/// once layout is complete.
#[derive(Clone, Debug)]
pub struct ITokenProps {
    pub region: IGlyphRegion,
    pub offset: IGlyphOffset,
}

impl FontTrack {
    /// Returns the default [`ITokenProps`] for any glyph in this track, ignoring per-token overrides.
    ///
    /// Use this when you need the track's baseline region and offset without consulting the
    /// per-token override table. If you are computing props for a specific token during layout
    /// packing, use [`props`](Self::props) instead.
    pub const fn as_token_props(&self) -> ITokenProps {
        ITokenProps {
            region: self.glyph_region,
            offset: self.offset,
        }
    }

    /// Returns the [`ITokenProps`] for a glyph in this track, applying any per-token overrides.
    ///
    /// If `overrides` is `Some`, each field (offset and region) is individually replaced only
    /// if the override provides a value for it; fields absent from the override fall back to the
    /// track defaults. If `overrides` is `None`, this is equivalent to [`as_token_props`](Self::as_token_props).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use raster_font::{core::*, meta::*};
    /// # let track: FontTrack = todo!();
    /// # let my_offset: IGlyphOffset = todo!();
    /// #
    /// // Override only the offset, keep the track's default region.
    /// let props = track.props(Some(&GlyphOverride { offset: Some(my_offset), region: None }));
    /// ```
    pub const fn props<'f>(&'f self, overrides: Option<&'f GlyphOverride>) -> ITokenProps {
        if let Some(GlyphOverride { offset, region }) = overrides {
            let offset = match offset.as_ref() {
                Some(&offset) => offset,
                None => self.offset,
            };
            let region = match region.as_ref() {
                Some(&region) => region,
                None => self.glyph_region,
            };
            ITokenProps { offset, region }
        } else {
            self.as_token_props()
        }
    }
}

/// Marker type indicating that a build phase has **not** yet been completed.
///
/// Used as a default type parameter on [`FontAtlasBuilder`] to prevent calling phase-gated
/// methods before their prerequisites have been satisfied. See also [`Populated`].
pub struct Unpopulated;
/// Marker type indicating that a build phase **has** been completed.
///
/// Once a phase transitions to `Populated`, the methods gated on that phase become available
/// on [`FontAtlasBuilder`]. See also [`Unpopulated`].
pub struct Populated;

mod _marker {
    use super::*;

    pub trait Marker {}
    impl Marker for Unpopulated {}
    impl Marker for Populated {}
    pub trait ImageMarker {}
    impl ImageMarker for PhantomData<Unpopulated> {}
    impl<T> ImageMarker for GlyphSheet<T> {}
}
use _marker::{ImageMarker, Marker};

/// A staged builder for constructing a [`RasterFont`].
///
/// The three type parameters enforce correct build order at compile time:
///
/// | Parameter         | Default                       | Meaning                                                |
/// | :---------------: | :---------------------------- | :----------------------------------------------------- |
/// | `LayoutPopulated` | [`Unpopulated`]               | Whether glyph regions have been computed from a layout |
/// | `CustomPopulated` | [`Unpopulated`]               | Whether custom/hand-specified glyphs have been added   |
/// | `Image`           | `PhantomData<Unpopulated>`    | The attached sprite sheet, or absent if not yet set    |
/// | `Named`           | [`Unpopulated`]               | Whether the font has been named                        |
///
/// Methods that require a specific phase to have been completed are only present on the
/// corresponding specialisation, so missing a step results in a compile error rather than a
/// runtime panic or silently producing an invalid font.
///
/// Construct a builder with [`default`](Self::default) or [`with_capacity`](Self::with_capacity),
/// then follow the build sequence described in the [module docs](self).
#[derive(Clone, Debug)]
pub struct FontAtlasBuilder<
    LayoutPopulated: Marker = Unpopulated,
    CustomPopulated: Marker = Unpopulated,
    Sheet: ImageMarker = PhantomData<Unpopulated>,
    Named: Marker = Unpopulated,
> {
    name: Option<String>,
    image: Sheet,
    scratch: Scratch,
    _state: PhantomData<(LayoutPopulated, CustomPopulated, Named)>,
}

/// Internal accumulator for glyph data during the build.
///
/// Stores the flat list of [`UTokenProps`] (indexed by [`AtlasIndex`]) and the map from
/// [`Sequence`] to [`AtlasIndex`] that is handed off to [`LigatureTree`] and [`RasterFont`].
///
/// [`LigatureTree`]: crate::tree::LigatureTree
#[derive(Clone, Default, Debug)]
pub(crate) struct Scratch {
    glyphs: Vec<UTokenProps>,
    sequence_map: HashMap<Sequence, AtlasIndex>,
}

/// A raw image paired with its pixel dimensions, used as the glyph sprite sheet.
///
/// The `image` field holds backend-specific pixel data (e.g. `Vec<u8>`, a file path, or a
/// Bevy `Handle<Image>`). The `size` field is required by the builder to compute tile positions
/// within the sheet during layout packing.
#[derive(Clone, Debug)]
pub struct GlyphSheet<Image> {
    /// The raw image data or handle for this sprite sheet.
    pub image: Image,
    /// The dimensions of the image in pixels.
    pub size: UGlyphSize,
}

impl Default for FontAtlasBuilder {
    fn default() -> Self {
        Self {
            name: None,
            image: PhantomData,
            scratch: Scratch::default(),
            _state: PhantomData,
        }
    }
}

impl<A: Marker, B: Marker, C: ImageMarker> FontAtlasBuilder<A, B, C, Unpopulated> {
    /// Assign a human-readable name to the font being built. This is optional but can be useful
    /// for debugging and error messages.
    #[inline]
    pub fn with_name(self, name: Option<String>) -> FontAtlasBuilder<A, B, C, Populated> {
        FontAtlasBuilder {
            name,
            image: self.image,
            scratch: self.scratch,
            _state: PhantomData,
        }
    }
}

impl<A: Marker, B: Marker, C: ImageMarker, Named: Marker> FontAtlasBuilder<A, B, C, Named> {
    #[doc(hidden)]
    #[inline]
    fn rebrand<X: Marker, Y: Marker>(self) -> FontAtlasBuilder<X, Y, C, Named> {
        FontAtlasBuilder {
            name: self.name,
            image: self.image,
            scratch: self.scratch,
            _state: PhantomData,
        }
    }

    /// Register a glyph in the builder, assigning it the next available [`AtlasIndex`].
    ///
    /// All sequences produced by iterating `token` are mapped to the new index, overwriting
    /// any existing entries for those sequences. This means later calls for the same sequence
    /// silently win — callers should ensure each sequence appears in at most one token.
    ///
    /// Called internally by [`uniform_layout`](Self::uniform_layout),
    /// [`dynamic_layout`](Self::dynamic_layout), and [`custom_glyphs`](Self::custom_glyphs).
    pub fn add_token(&mut self, token: &Token, props: UTokenProps) {
        let index = AtlasIndex(self.scratch.glyphs.len());
        self.scratch.glyphs.push(props);
        self.scratch
            .sequence_map
            .extend(token.iter().cloned().zip(std::iter::repeat(index)));
    }

    /// Look up the [`UTokenProps`] for a sequence already registered in the builder.
    ///
    /// Returns `None` if the sequence has not been registered yet. Primarily used by
    /// [`custom_glyphs`](FontAtlasBuilder::custom_glyphs) to resolve relative glyph references.
    pub fn get_glyph_by_sequence(&self, token: &Sequence) -> Option<&UTokenProps> {
        // let glyph = self.lookup.get(token)?;
        let index = self.scratch.sequence_map.get(token)?;
        self.scratch.glyphs.get(index.0)
    }
}

impl FontAtlasBuilder {
    /// Create a builder with pre-allocated capacity.
    ///
    /// Use this when the number of unique glyphs and sequences is known ahead of time to avoid
    /// reallocations during layout population.
    ///
    /// - `num_unique_glyphs` — expected number of distinct glyph entries (i.e. unique [`AtlasIndex`]es).
    /// - `num_unique_sequences` — expected total number of sequence strings mapped to glyphs.
    ///
    /// See also [`FontLayout::unique`] and [`Self::from(unique)`](Unique) for convenient
    /// alternatives to this method.
    #[inline]
    pub fn with_capacity(num_unique_glyphs: usize, num_unique_sequences: usize) -> Self {
        Self {
            name: None,
            scratch: Scratch {
                glyphs: Vec::with_capacity(num_unique_glyphs),
                sequence_map: HashMap::with_capacity(num_unique_sequences),
            },
            _state: PhantomData,
            image: PhantomData,
        }
    }
}

impl<Named: Marker> FontAtlasBuilder<Unpopulated, Unpopulated, PhantomData<Unpopulated>, Named> {
    /// Attach a glyph sprite sheet to this builder, advancing the `Image` state parameter.
    ///
    /// Must be called before [`populate_layout`](FontAtlasBuilder::populate_layout). The `size`
    /// field of the [`GlyphSheet`] is used to compute tile positions during layout packing.
    #[inline]
    pub fn with_image<Image>(
        self,
        image: GlyphSheet<Image>,
    ) -> FontAtlasBuilder<Unpopulated, Unpopulated, GlyphSheet<Image>, Named>
    where
        GlyphSheet<Image>: ImageMarker,
    {
        FontAtlasBuilder {
            name: self.name,
            image,
            scratch: self.scratch,
            _state: PhantomData,
        }
    }
}

/// Constructs a [`FontAtlasBuilder`] pre-sized to fit the glyph counts described by a [`Unique`]
/// layout.
impl From<Unique<'_>> for FontAtlasBuilder {
    #[inline]
    fn from(unique: Unique) -> Self {
        Self::with_capacity(unique.num_regions, unique.sequences.len())
    }
}

/// Compute the unsigned pixel region of a glyph inside a texture atlas.
///
/// Translates the per-track `region` (relative to the tile's top-left corner) by `tile_start`
/// (the top-left corner of the tile within the full atlas image) and converts the result to
/// an unsigned rectangle.
#[inline]
#[must_use]
fn extract_region(tile_start: IGlyphOffset, region: IGlyphRegion) -> UGlyphRegion {
    let region = IGlyphRegion {
        min: tile_start + region.min,
        max: tile_start + region.max,
    };
    region.as_urect()
}

impl<Image, Named: Marker> FontAtlasBuilder<Unpopulated, Unpopulated, GlyphSheet<Image>, Named>
where
    GlyphSheet<Image>: ImageMarker,
{
    /// Populate glyph regions and the sequence map from a [`FontLayout`], dispatching the
    /// appropriate packing strategy based on the layout's [`PackingMode`].
    ///
    /// This is the primary entry point for layout population. Advances `LayoutPopulated` to
    /// [`Populated`], enabling the subsequent [`custom_glyphs`](FontAtlasBuilder::custom_glyphs)
    /// and [`build`](FontAtlasBuilder::build) steps.
    pub fn populate_layout(
        self,
        layout: &FontLayout,
    ) -> FontAtlasBuilder<Populated, Unpopulated, GlyphSheet<Image>, Named> {
        match layout.packing_mode() {
            PackingMode::Uniform { track } => self.uniform_layout(layout, track),
            PackingMode::Dynamic { tracks } => self.dynamic_layout(layout, tracks),
        }
    }

    /// Populate glyph regions for a **uniform** (fixed-tile) packing mode.
    ///
    /// In uniform mode, all glyphs share a single [`FontTrack`] and are laid out left-to-right
    /// (wrapping to the next row) on a fixed grid whose tile size is `track.grid_tile_size`. The
    /// tile index for each glyph in `layout.ord_layout()` directly determines its position in the
    /// atlas image.
    ///
    /// Per-token overrides from the layout are respected: if a token has an override entry, its
    /// offset and/or region replace the track defaults for that token only.
    pub fn uniform_layout(
        mut self,
        layout: &FontLayout,
        track: &FontTrack,
    ) -> FontAtlasBuilder<Populated, Unpopulated, GlyphSheet<Image>, Named> {
        for (tile_index, token) in layout.ord_layout().iter().enumerate() {
            // Use preferences for this token
            let ITokenProps { offset, region } =
                track.props(token.first().and_then(|t| layout.get_override(t)));

            // Extract the glyph region from the texture.
            let props = UTokenProps {
                offset,
                region: {
                    let tile_start_x = (tile_index as u32) * track.grid_tile_size.x;
                    let tile_start = UGlyphSize::new(
                        (tile_start_x) % self.image.size.x,
                        (tile_start_x) / self.image.size.x * track.grid_tile_size.y,
                    )
                    .as_ivec2();

                    extract_region(tile_start, region)
                },
            };

            // Add the glyph to the atlas and lookup table.
            self.add_token(token, props);
        }

        self.rebrand()
    }

    /// Populate glyph regions for a **dynamic** (variable-tile) packing mode.
    ///
    /// In dynamic mode, different groups of glyphs may use different [`FontTrack`]s with
    /// distinct tile sizes. Tracks are keyed by a sentinel *head token* in `tracks`; the builder
    /// switches to a new track each time it encounters a head token in the ordered layout, and
    /// advances a cursor that wraps to the next row whenever the current row is full.
    ///
    /// Tokens that appear before any head token are silently skipped (no track is active yet).
    ///
    /// Per-token overrides from the layout are respected for each token, as in uniform mode.
    /// [`uniform_layout`](Self::uniform_layout).
    pub fn dynamic_layout(
        mut self,
        layout: &FontLayout,
        tracks: &HashMap<Token, FontTrack>,
    ) -> FontAtlasBuilder<Populated, Unpopulated, GlyphSheet<Image>, Named> {
        // todo: Could lift this into the builder and allow multiple dynamic layouts in one font.
        struct DynamicPackingCursor {
            pos: IGlyphOffset,
            tallest_in_row: i32,
            max_x: i32,
        }

        impl DynamicPackingCursor {
            const fn new(image_size: UGlyphSize) -> Self {
                Self {
                    pos: IGlyphOffset::ZERO,
                    tallest_in_row: 0,
                    max_x: image_size.x as i32,
                }
            }

            fn advance(&mut self, track: &FontTrack) {
                let tile = track.grid_tile_size.as_ivec2();

                if self.pos.x + tile.x > self.max_x {
                    self.pos.x = 0;
                    self.pos.y += self.tallest_in_row;
                    self.tallest_in_row = 0;
                }

                self.tallest_in_row = self.tallest_in_row.max(tile.y);
                self.pos.x += tile.x;
            }
        }

        let mut current_track: Option<&FontTrack> = None;
        let mut cursor = DynamicPackingCursor::new(self.image.size);
        // todo: end todo

        for token in layout.ord_layout().iter() {
            // switch tracks when we encounter the head token for a track
            if let Some(track) = tracks.get(token) {
                current_track = Some(track);
            }
            // if we're not in a track, we can't place any glyphs, so skip until we find a track
            let track = match current_track {
                Some(track) => track,
                None => continue,
            };

            // Use preferences for this token
            let ITokenProps { offset, region } =
                track.props(token.first().and_then(|t| layout.get_override(t)));

            // Extract the glyph region from the texture.
            let props = UTokenProps {
                offset,
                region: {
                    let tile_start = cursor.pos;
                    cursor.advance(track);
                    extract_region(tile_start, region)
                },
            };

            // Add the glyph to the atlas and lookup table.
            self.add_token(token, props);
        }

        self.rebrand()
    }
}

impl<Sheet: ImageMarker, Named: Marker> FontAtlasBuilder<Populated, Unpopulated, Sheet, Named> {
    /// Register hand-specified [`CustomGlyph`]s that are not part of the main layout.
    ///
    /// Custom glyphs come in two flavours:
    ///
    /// - [`CustomGlyph::Absolute`] — the region and offset are given directly in atlas coordinates.
    /// - [`CustomGlyph::Relative`] — the region is expressed relative to a *reference sequence*
    ///   that must already exist in the builder. This is useful for sub-glyphs that share a
    ///   sprite sheet tile with a base glyph (e.g. combining a [diacritic]).
    ///
    /// # Errors
    ///
    /// Returns [`UnknownSequence`] if a relative custom glyph references a
    /// sequence that has not been registered by the preceding layout population step.
    ///
    /// # Notes
    ///
    /// This method is only available after `LayoutPopulated = Populated` because relative
    /// references must be resolvable against already-registered glyphs.
    ///
    /// [diacritic]: https://en.wikipedia.org/wiki/Diacritic
    pub fn custom_glyphs<'a>(
        mut self,
        iter: impl IntoIterator<Item = (&'a Token, &'a CustomGlyph)>,
    ) -> Result<FontAtlasBuilder<Populated, Populated, Sheet, Named>, UnknownSequence> {
        for (token, sub_glyph) in iter {
            #[allow(clippy::match_ref_pats)]
            let props = match sub_glyph {
                &CustomGlyph::Absolute { offset, region } => UTokenProps { offset, region },
                &CustomGlyph::Relative {
                    offset,
                    ref reference,
                    region,
                } => {
                    let Some(reference_props) = self.get_glyph_by_sequence(reference) else {
                        return Err(UnknownSequence(reference.clone()));
                    };

                    UTokenProps {
                        offset,
                        region: extract_region(reference_props.region.min.as_ivec2(), region),
                    }
                }
            };

            self.add_token(token, props);
        }

        Ok(self.rebrand())
    }
}

/// Intermediate font representation passed from the builder to a [`BackendBuilder`].
///
/// Contains data produced by the layout population phase. [`BackendBuilder::build_resources`]
/// consumes this to produce final [`Backend::Resources`].
pub struct RawFont<Sheet> {
    /// The glyph sprite sheet (image + dimensions).
    pub sheet: GlyphSheet<Sheet>,
    /// A flat list of [`UTokenProps`], indexed by [`AtlasIndex`].
    pub glyphs: Vec<UTokenProps>,
    #[allow(rustdoc::private_intra_doc_links)]
    /// See [`Scratch::max_glyph_height`] for how this is computed.
    pub computed_height: u32,
}

impl<C: Marker, Image, Named: Marker> FontAtlasBuilder<Populated, C, GlyphSheet<Image>, Named>
where
    GlyphSheet<Image>: ImageMarker,
{
    /// Finalize the builder and construct a [`RasterFont`].
    ///
    /// Computes the font's line height, invokes the [`BackendBuilder`] to finalize resources, and
    /// compiles the internal [`LigatureTree`]. All three steps must succeed for a font to be
    /// returned.
    ///
    /// # Errors
    ///
    /// Returns a [`FontBuilderError`] wrapping one of:
    ///
    /// - [`EmptyFont`]: no glyphs were registered.
    /// - [`BackendBuilderError`](errors::FontBuilderError::BackendBuilderError): the backend
    ///   returned an error.
    /// - [`LigatureBindingError`](errors::FontBuilderError::LigatureBindingError): the
    ///   Aho-Corasick automaton failed to compile.
    ///
    /// [`LigatureTree`]: crate::tree::LigatureTree
    pub fn build<B: Backend, Ctx: BackendBuilder<Backend = B, Error: Error, Sheet = Image>>(
        self,
        resource_builder: Ctx,
    ) -> Result<RasterFont<B>, FontBuilderError<Ctx::Error>> {
        let computed_height = self
            .scratch
            .max_glyph_height()
            .ok_or(EmptyFont(PhantomData))?;
        let resources = resource_builder
            .build_resources(RawFont {
                glyphs: self.scratch.glyphs,
                sheet: self.image,
                computed_height,
            })
            .map_err(FontBuilderError::BackendBuilderError)?;

        RasterFont::new(
            self.name,
            computed_height,
            resources,
            self.scratch.sequence_map,
        )
        .map_err(FontBuilderError::LigatureBindingError)
    }
}

use errors::{EmptyFont, FontBuilderError, UnknownSequence};

/// Error types that can be produced during font construction.
pub mod errors {
    use crate::core::Sequence;
    use std::{
        error::Error,
        fmt::{Display, Formatter, Result},
        marker::PhantomData,
    };

    /// The error type returned by [`FontAtlasBuilder::build`](super::FontAtlasBuilder::build).
    ///
    /// Each variant corresponds to a distinct failure mode in the build pipeline:
    ///
    /// - [`EmptyFont`](Self::EmptyFont) — `build` was called with zero registered glyphs.
    /// - [`UnknownSequence`] — a [`CustomGlyph::Relative`](crate::meta::CustomGlyph::Relative)
    ///   referenced a sequence that was not found in the layout.
    /// - [`LigatureBindingError`](Self::LigatureBindingError) — the Aho-Corasick automaton
    ///   failed to compile from the registered sequence set.
    /// - [`BackendBuilderError`](Self::BackendBuilderError) — the backend's
    ///   [`build_resources`](crate::backend::BackendBuilder::build_resources) call failed.
    #[derive(Debug)]
    pub enum FontBuilderError<E: Error> {
        /// No glyphs were registered before `build` was called.
        EmptyFont,
        /// A custom glyph's relative reference could not be resolved.
        UnknownSequence(UnknownSequence),
        /// The Aho-Corasick automaton failed to compile.
        LigatureBindingError(crate::tree::BuildError),
        /// Custom error returned by [`BackendBuilder::build_resources`] when constructing
        /// backend-specific resources for a raster font.
        ///
        /// [`BackendBuilder::build_resources`]: crate::backend::BackendBuilder::build_resources
        BackendBuilderError(E),
    }

    impl<E: Error> From<EmptyFont> for FontBuilderError<E> {
        #[inline]
        fn from(_: EmptyFont) -> Self {
            FontBuilderError::EmptyFont
        }
    }

    impl<E: Error> From<UnknownSequence> for FontBuilderError<E> {
        #[inline]
        fn from(e: UnknownSequence) -> Self {
            FontBuilderError::UnknownSequence(e)
        }
    }

    impl<E: Error> Error for FontBuilderError<E> {}
    impl<E: Error> Display for FontBuilderError<E> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            match self {
                Self::EmptyFont => EmptyFont(PhantomData).fmt(f),
                FontBuilderError::UnknownSequence(a) => a.fmt(f),
                FontBuilderError::LigatureBindingError(a) => a.fmt(f),
                FontBuilderError::BackendBuilderError(e) => {
                    write!(
                        f,
                        "Failed to build backend-specific resources for a raster font: {e}"
                    )
                }
            }
        }
    }

    /// Error returned when [`build`](super::FontAtlasBuilder::build) is called without any
    /// registered glyphs.
    #[derive(Debug)]
    pub struct EmptyFont(pub(super) PhantomData<()>);

    impl Error for EmptyFont {}
    impl Display for EmptyFont {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write!(f, "Will not build empty font (0 glyphs).")
        }
    }

    /// Error returned when a [`CustomGlyph::Relative`](crate::meta::CustomGlyph::Relative)
    /// references a sequence that does not exist in the layout.
    ///
    /// Contains the unresolved [`Sequence`] for diagnostic purposes.
    ///
    /// For simpliciy, the Sequence is cloned when this error is constructed. In practice this
    /// error is only be returned when their is an authoring mistake in the layout, so production
    /// code should never see this error unless the layout is being generated dynamically from
    /// untrusted input.
    #[derive(Debug)]
    pub struct UnknownSequence(pub(super) Sequence);

    impl Error for UnknownSequence {}
    impl Display for UnknownSequence {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write!(
                f,
                "A custom glyph references a sequence that does not exist in the layout: {}",
                self.0
            )
        }
    }
}

impl Scratch {
    /// Returns an iterator of `(sequence, props)` pairs for all registered glyphs.
    ///
    /// Sequences whose [`AtlasIndex`] no longer points to a valid entry (which should not occur
    /// under normal use) are silently filtered out.
    #[allow(dead_code)]
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&Sequence, &UTokenProps)> {
        self.sequence_map
            .iter()
            .filter_map(|(seq, index)| self.glyphs.get(index.0).map(|props| (seq, props)))
    }

    /// Returns an iterator of [`UTokenProps`] for all sequences in `self`.
    ///
    /// Multiple sequences that map to the same [`AtlasIndex`] will yield the same props more
    /// than once (once per sequence entry).
    #[inline]
    pub fn iter_props(&self) -> impl Iterator<Item = &UTokenProps> {
        self.sequence_map
            .values()
            .filter_map(|index| self.glyphs.get(index.0))
    }

    /// Returns the maximum effective glyph height across all registered glyphs.
    ///
    /// The effective height of a glyph is `region.height() + offset.y` (saturating). This is
    /// used by [`FontAtlasBuilder::build`](FontAtlasBuilder::build) to compute the font's
    /// line height.
    ///
    /// Returns `None` if no glyphs have been registered yet.
    #[inline]
    pub fn max_glyph_height(&self) -> Option<u32> {
        self.iter_props()
            .map(|g| g.region.height().saturating_add_signed(g.offset.y))
            .max()
    }
}
