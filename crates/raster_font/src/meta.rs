//! Serialization types for the raster font asset format.
//!
//! Most users will not need to interact with this module directly.
//!
//! # Format
//!
//! A raster font asset is a TOML file with the following top-level shape:
//!
//! ```toml
//! layout = "abc$(->|=>)xyz"   # OrdTokenLayout string
//! image  = "font.png"         # relative path to the font texture
//!
//! [pack]                      # PackingMode — uniform grid or dynamic tracks
//! size   = [8, 8]
//! region = { min = [0, 0], max = [8, 8] }
//!
//! [override."a"]              # Optional per-glyph overrides (region, offset)
//! region = [[0, 0], [6, 8]]
//!
//! [extract."$(:))"]           # Manually extracted glyphs (ligatures, icons, etc.)
//! # ref    = "a"                # Extract relative to the region of "a" in the atlas
//! region = [[128, 0], [136, 8]]
//! ```
//!
//! See [`PackingMode`] for details on uniform vs. dynamic atlas layouts, and
//! [`CustomGlyph`] for how to manually extract glyph regions.
use std::{
    iter::{Chain, FlatMap},
    path::PathBuf,
    slice::Iter,
};

use serde::{Deserialize, Serialize};

use crate::{
    collections::{HashMap, hash_map::Keys},
    core::{IGlyphOffset, IGlyphRegion, UGlyphRegion, UGlyphSize, Unique},
    layout::OrdTokenLayout,
    token::{Sequence, Token},
};

/// An image-based font format designed for pixel art games, supporting dynamic glyph packing,
/// manual region extraction, glyph nudging, and ligatures through a flexible token system.
///
/// See the [module-level documentation](crate::meta) for details on the asset format and how to
/// write font metadata files.
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct FontMeta {
    /// A human-readable name for the font.
    pub name: Option<String>,
    /// Relative path to the font texture.
    #[serde(alias = "texture")]
    pub image: PathBuf,
    /// The layout of the font, defining the mapping of characters to glyphs and their properties.
    #[serde(flatten)]
    pub layout: FontLayout,
}

/// A raster font layout, defining the mapping of characters to glyphs and their properties.
#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct FontLayout {
    /// The characters included in the font, in the order they appear in the texture atlas.
    #[serde(rename = "layout")]
    pub layout_tokens: OrdTokenLayout,
    /// Packing mode for the font.
    #[serde(rename = "pack")]
    pub packing_mode: PackingMode,
    /// Overrides for specific sequences, allowing for custom regions and baselines on a per-glyph basis.
    ///
    /// ## Note
    ///
    /// Overrides only need to be specified for the first `Sequence` of a `Token`.
    ///
    /// ### Example
    /// ```toml
    /// layout = "abc$(def|ghi)"
    ///
    /// [override."a"] # override "a" only.
    ///
    /// [override."abc"] # does not exist, silently ignored.
    ///
    /// [override."def"] # override "def" and "ghi".
    /// ```
    #[serde(default, rename = "override", alias = "overrides")]
    pub sequence_overrides: HashMap<Sequence, GlyphOverride>,
    /// Manually extracted glyphs that don't fit the regular packing mode, such as ligatures or control characters.
    #[serde(default, rename = "extract")]
    pub manual_tokens: HashMap<Token, CustomGlyph>,
}

/// Determines how the atlas builder's *view* moves after analyzing a token's glyph region.
///
/// - *view*: The region of the font texture that a builder is analyzing
///   for glyph extraction. After analyzing a token, the builder moves the view according
///   to the packing mode before analyzing the next token.
///
/// Glyphs may capture pixels outside of their grid tile.
///
/// Glyphs that don't fit the regular packing mode can be included as manually extracted sequences
/// in a [`FontLayout`], which specify their own regions and don't affect the builder's view.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PackingMode {
    /// Expects atlas glyphs to be packed in a uniform grid. The builder moves its view by
    /// [`FontTrack::grid_tile_size`] after analyzing each token.
    ///
    /// # Example
    ///
    /// ```toml
    /// [pack]
    /// size   = [16, 16]
    /// region = [[1, 5], [7, 12]]
    /// offset = [0, 2]
    /// ```
    ///
    /// ## Notes
    ///
    /// - The size of the image does **not need to be an exact multiple** of the grid tile size.
    Uniform {
        #[serde(flatten)]
        track: FontTrack,
    },
    /// Automatically change font tracks when the head token of a track is encountered.
    ///
    /// The builder moves its view according to the current track until it encounters the head token
    /// of another track; then, it switches to that track and moves according to the new track's
    /// properties.
    ///
    /// The tallest glyph in a track determines the line height for that track. It is not currently
    /// possible to achieve nested grids with this packing mode, but it may be added in the future
    /// if there is demand for it.
    ///
    /// # Example
    ///
    /// ```toml
    /// [pack."a"]
    /// size = [16, 16],
    /// # no region defaults to full tile.
    /// # no offset defaults to [0, 0]
    ///
    /// [pack."A"]
    /// size = [16, 20],
    /// ```
    Dynamic {
        #[serde(flatten)]
        tracks: HashMap<Token, FontTrack>,
    },
}

/// A font track serves as a moving template when constructing a [`RawFont`].
///
/// It provides default properties for glyphs defined in an [`OrdTokenLayout`], and determines
/// the view and movement of the atlas builder as it iterates through the tokens in the layout.
///
/// [`RawFont`]: crate::builder::RawFont
#[derive(Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct FontTrack {
    /// The size of a tile that contains a single glyph, in pixels.
    ///
    /// This must contain the region of the glyph that is expected to be visible when rendered.
    #[serde(alias = "size")]
    pub grid_tile_size: UGlyphSize,
    /// The region of the tile that contains the visible part of the glyph, in pixels.
    /// This can be overwritten on a per-glyph basis to allow for different sized glyphs or to add
    /// or remove padding around specific characters.
    #[serde(alias = "region")]
    pub glyph_region: IGlyphRegion,
    /// Shift the glyph by a certain amount of pixels in the x and y directions when rendering.
    #[serde(default)]
    pub offset: IGlyphOffset,
}

/// Overrides for specific sequences, allowing for custom regions and baselines on a per-glyph basis.
///
/// ## Note
///
/// Overrides only need to be specified for the first `Sequence` of a `Token`.
///
/// ### Example
/// ```toml
/// layout = "abc$(def|ghi)"
///
/// [override."a".region] # override "a" only.
/// min = [0, 0]
/// max = [6, 8]
///
/// [override."abc"] # does not exist, silently ignored.
/// # *skipped*
///
/// [override."def".offset] # override "def" and "ghi" with the same offset.
/// x = 0
/// y = 2
/// ```
#[derive(Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct GlyphOverride {
    /// The region of the tile that contains the visible part of the glyph, in pixels.
    #[serde(default)]
    pub region: Option<IGlyphRegion>,
    /// Shift the glyph by a certain amount of pixels in the x and y directions when rendering.
    #[serde(default)]
    pub offset: Option<IGlyphOffset>,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CustomGlyph {
    Relative {
        /// The first sequence in the token to use as a reference point for the glyph region.
        ///
        /// The region of the glyph will be defined relative to the position of this reference
        /// glyph in the atlas.
        #[serde(alias = "ref")]
        reference: Sequence,
        /// The region of the image that contains the visible part of the glyph, in pixels, relative
        /// to the reference glyph's anchor point (the top left corner of the glyph region).
        region: IGlyphRegion,
        /// Shift the glyph by a certain amount of pixels in the x and y directions when rendering.
        #[serde(default)]
        offset: IGlyphOffset,
    },
    Absolute {
        /// The region of the image that contains the visible part of the glyph, in pixels.
        region: UGlyphRegion,
        /// Shift the glyph by a certain amount of pixels in the x and y directions when rendering.
        #[serde(default)]
        offset: IGlyphOffset,
    },
}

impl Default for PackingMode {
    fn default() -> Self {
        Self::Uniform {
            track: Default::default(),
        }
    }
}

type GlyphIter<'a> = Chain<Iter<'a, Token>, Keys<'a, Token, CustomGlyph>>;

impl FontLayout {
    /// Returns a reference to the packing mode of the font layout, which determines how
    /// the atlas builder's view moves after analyzing a token's glyph region.
    #[inline]
    pub fn packing_mode(&self) -> &PackingMode {
        &self.packing_mode
    }

    /// Get override properties for a [`Sequence`], if it has any.
    ///
    /// Returns `None` if the token is not present in the layout or if it has no overrides.
    #[inline]
    pub fn get_override(&self, token: &Sequence) -> Option<&GlyphOverride> {
        self.sequence_overrides.get(token)
    }

    /// Returns the internal token layout as an immutable slice. This does **not include custom
    /// glyphs**.
    ///
    /// Use [`iter_tokens`] to get an iterator over all tokens, including both the ordered token
    /// layout and any custom glyphs.
    ///
    /// [`iter_tokens`]: Self::iter_tokens
    #[inline]
    pub fn ord_layout(&self) -> &[Token] {
        self.layout_tokens.as_ref()
    }

    /// Returns an immutable reference to the map of custom glyphs, which are not
    /// included in the main token layout.
    ///
    /// These are typically used for glyphs that don't fit the regular packing mode.
    #[inline]
    pub fn custom(&self) -> &HashMap<Token, CustomGlyph> {
        &self.manual_tokens
    }

    /// Returns an iterator over the all tokens in `self`.
    ///
    /// This includes both the ordered token layout and any custom glyphs.
    #[inline]
    pub fn iter_tokens(&'_ self) -> GlyphIter<'_> {
        self.layout_tokens
            .iter_tokens()
            .chain(self.manual_tokens.keys())
    }

    /// Returns the a flattened iterator of all sequences in `self`, including tokens in the ordered
    /// token layout and any extracted custom glyphs.
    #[inline]
    #[allow(clippy::type_complexity)]
    pub fn iter_sequences(
        &'_ self,
    ) -> FlatMap<GlyphIter<'_>, Iter<'_, Sequence>, fn(&Token) -> Iter<Sequence>> {
        self.iter_tokens().flat_map(Token::iter)
    }

    /// Returns the number of glyph regions and unique sequences in this layout.
    ///
    /// This is used to pre-allocate the correct amount of capacity for a font builder.
    #[doc(alias = "glyph count")]
    #[doc(alias = "region count")]
    #[doc(alias = "unique sequence")]
    #[inline]
    pub fn unique<'a>(&'a self) -> Unique<'a> {
        self.iter_tokens().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde() {
        let meta = FontMeta {
            name: Some("Test Font".into()),
            image: "font.png".into(),
            layout: FontLayout {
                layout_tokens: r#"0123456789 ABCDEFG.$($btn_face_south|$btn_dpad_south)"#
                    .parse()
                    .unwrap(),
                packing_mode: PackingMode::Uniform {
                    track: FontTrack {
                        grid_tile_size: UGlyphSize::new(8, 8),
                        glyph_region: IGlyphRegion::new(0, 0, 8, 8),
                        offset: IGlyphOffset::new(0, 0),
                    },
                },
                sequence_overrides: HashMap::from([(
                    ' '.into(),
                    GlyphOverride {
                        region: Some(IGlyphRegion::new(0, 0, 4, 8)),
                        offset: None,
                    },
                )]),
                manual_tokens: HashMap::from([(
                    "$($btn_face_south)".parse().unwrap(),
                    CustomGlyph::Relative {
                        reference: '0'.into(),
                        region: IGlyphRegion::new(1, 5, 7, 12),
                        offset: IGlyphOffset::new(0, 2),
                    },
                )]),
            },
        };

        println!("Original:\n{}", toml::to_string_pretty(&meta).unwrap());

        let ex = r#"
            layout = "0123456789 ABCDEFG.$($btn_face_south|$btn_dpad_south)"
            image = "font.png"

            [pack]
            size = [8, 8]
            region = { min = [0, 0], max = [8, 8] }
            # offset = [20000, 0]

            [override." "]
            region = [[0, 0], [4, 8]]

            [extract."$($btn_face_south)"]
            ref = "0"
            region = [[1, 5], [7, 12]]
            offset = [0, 2]
        "#;

        let deserialized: FontMeta = toml::from_str(ex).unwrap();

        assert_eq!(&meta.image, &deserialized.image);
        assert_eq!(
            &meta.layout.sequence_overrides,
            &deserialized.layout.sequence_overrides
        );

        println!(
            "Deserialized:\n{}",
            toml::to_string_pretty(&deserialized).unwrap()
        );

        assert_eq!(
            &meta.layout.manual_tokens,
            &deserialized.layout.manual_tokens
        );

        assert_eq!(meta.layout.packing_mode, deserialized.layout.packing_mode);
        assert_eq!(meta.layout.layout_tokens, deserialized.layout.layout_tokens);
    }
}
