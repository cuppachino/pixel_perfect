//! Bevy integration for [`RasterFont`].
//!
//! # Fonts as assets
//!
//! A [`RasterFont<BevyBackend>`] is a Bevy [`Asset`] backed by two sub-assets:
//! - [`Handle<Image>`]: points to the packed glyph sheet texture
//! - [`Handle<TextureAtlasLayout>`]: describes per-glyph UV regions
//!
//! Both handles are stored in [`BevyAtlasResources`] and visited automatically
//! by the asset dependency system so Bevy can track loading state correctly.
//!
//! # Resolving glyph data
//!
//! [`Assets<TextureAtlasLayout>`] implements [`FontResourceProvider`], so you can
//! upgrade a borrowed font into a [`RasterFontCtx`](crate::backend::RasterFontCtx)
//! by calling [`RasterFont::upgrade`](crate::backend::RasterFont::upgrade):
//!
//! ```rust
//! # use raster_font::{backend::prelude::*, tree::InputResolver};
//! # use raster_font::bevy_backend::BevyBackend;
//! # use bevy_asset::Assets;
//! # use bevy_image::TextureAtlasLayout;
//! #
//! fn draw(
//!     font: &RasterFont<BevyBackend>,
//!     atlases: &Assets<TextureAtlasLayout>,
//! ) {
//!     let Ok(ctx) = font.upgrade(atlases) else { return };
//!
//!     for glyph in ctx.valid("Hello!") {
//!         // draw glyph
//!     }
//! }
//! ```
//!
//! The context is valid for the duration of the borrow and exposes the full
//! [`InputResolver`](crate::tree::InputResolver) API (`find`, `valid`, etc.).
//!
//! # Loading
//!
//! Fonts are loaded with the [`RasterFontAssetLoaderPlugin`](loader::RasterFontAssetLoaderPlugin).
//! The loader drives a [`BackendBuilder`] pipeline that:
//!
//! 1. Receives a [`RawFont<Handle<Image>>`](crate::builder::RawFont) from the
//!    [`FontAtlasBuilder`](crate::builder::FontAtlasBuilder).
//! 2. Constructs a [`TextureAtlasLayout`] from the packed glyph regions.
//! 3. Registers it as the labeled sub-asset
//!    `"raster_font::texture_atlas_layout"`.
//! 4. Stores the resulting handles in [`BevyAtlasResources`].
//!
//! # Texture sampling
//!
//! [`FontSampler`] controls how the glyph texture is sampled during rendering.
//! The default is nearest-neighbour filtering, which preserves sharp edges on
//! pixel fonts. Override it via [`RasterFontLoaderSettings`]
//! before loading if a different sampler is needed.
//!
//! # See also
//!
//! - [`BevyBackend`]
//! - [`RasterFont`]
//! - [`FontResourceProvider`]
//!
//! [`Handle`]: bevy_asset::Handle
//! [`Assets<T>`]: bevy_asset::Assets
//! [`Image`]: bevy_image::Image
//! [`TextureAtlasLayout`]: bevy_image::TextureAtlasLayout
pub mod loader;

pub mod prelude {
    pub use super::{
        BevyBackend, FontSampler,
        loader::{RasterFontAssetLoaderPlugin, RasterFontLoaderSettings},
    };
    pub use crate::tree::InputResolver;

    pub type RasterFont = crate::backend::RasterFont<BevyBackend>;
}

use std::{error::Error, marker::PhantomData};

use bevy_asset::{LoadContext, VisitAssetDependencies, prelude::*};
use bevy_ecs::{component::Component, resource::Resource};
use bevy_image::{Image, ImageSampler, TextureAtlasLayout};
use bevy_reflect::{Reflect, TypePath};
use serde::{Deserialize, Serialize};

use crate::backend::{bevy_backend::loader::RasterFontLoaderSettings, prelude::*};

/// Zero-sized tag struct that identifies the Bevy rendering backend.
///
/// Pass this as the type parameter of [`RasterFont<BevyBackend>`] to produce a
/// font asset that integrates with Bevy's asset and rendering systems.
///
/// # Backend type family
///
///| Associated type        | Concrete type          |
///| :--------------------- | ---------------------- |
///| `Backend::Atlas`       | [`TextureAtlasLayout`] |
///| `Backend::Image`       | [`Image`]              |
///| `Backend::Resources`   | [`BevyAtlasResources`] |
///
/// [`Backend::Atlas`]: crate::backend::Backend::Atlas
/// [`Backend::Image`]: crate::backend::Backend::Image
/// [`Backend::Resources`]: crate::backend::Backend::Resources
#[derive(Clone, Copy, Debug, Default, Reflect, Serialize, Deserialize)]
pub struct BevyBackend;

impl Backend for BevyBackend {
    type Resources = BevyAtlasResources;
}

impl<B> Asset for RasterFont<B>
where
    B: Backend + Clone + std::fmt::Debug + Send + Sync + TypePath,
    <B as Backend>::Resources: Clone + std::fmt::Debug + Send + Sync + VisitAssetDependencies,
{
}

impl<B: Backend<Resources: VisitAssetDependencies>> VisitAssetDependencies for RasterFont<B> {
    fn visit_dependencies(&self, visit: &mut impl FnMut(bevy_asset::UntypedAssetId)) {
        self.resources.visit_dependencies(visit);
    }
}

/// Bevy-specific resources stored inside every [`RasterFont<BevyBackend>`].
#[derive(Clone, Debug, Asset, TypePath)] //? Asset derives VisitAssetDependencies
pub struct BevyAtlasResources {
    pub sampler: FontSampler,
    pub image: Handle<Image>,
    pub offsets: Vec<IGlyphOffset>,
    pub regions: Handle<TextureAtlasLayout>,
}

/// [`BackendBuilder`] implementation that converts a [`RawFont<Handle<Image>>`]
/// into [`BevyAtlasResources`] during asset loading.
///
/// Constructed by the asset loader with a reference to the active
/// [`LoadContext`] (needed to register the atlas layout as a labeled sub-asset)
/// and the user-provided [`RasterFontLoaderSettings`].
///
/// You do not normally construct this directly — it is created internally by
/// [`RasterFontAssetLoaderPlugin`](loader::RasterFontAssetLoaderPlugin).
pub struct BuildBevyFont<'a, 'b> {
    pub load_context: &'a mut LoadContext<'b>,
    pub settings: &'a RasterFontLoaderSettings,
}

/// Infallible error type for [`BuildBevyFont`].
///
/// [`labeled_asset_scope`](LoadContext::labeled_asset_scope) is documented as
/// returning an error only when the provided closure panics, which cannot happen
/// here. This type is therefore effectively unreachable in practice, but is
/// required to satisfy the [`BackendBuilder::Error`] associated type.
#[derive(Debug)]
#[doc(hidden)]
pub struct LabeledAssetScopeError {
    _sealed: PhantomData<()>,
}
impl Error for LabeledAssetScopeError {}
impl std::fmt::Display for LabeledAssetScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to create a labeled asset scope for a raster font's texture atlas layout. This is an internal error that should not occur under normal circumstances. Please report this to the maintainers."
        )
    }
}

impl BackendBuilder for BuildBevyFont<'_, '_> {
    type Backend = BevyBackend;
    type Error = LabeledAssetScopeError;
    /// The glyph sheet is already loaded as a Bevy asset before the font builder
    /// runs, so the raw image is passed in as a [`Handle<Image>`] rather than
    /// raw pixel data.
    type Sheet = Handle<Image>;

    /// Converts a [`RawFont<Handle<Image>>`] into [`BevyAtlasResources`].
    ///
    /// Steps performed:
    /// 1. Unpacks per-glyph [`UTokenProps`] into a `textures` vec (UV rects)
    ///    and a parallel `offsets` vec (signed draw offsets).
    /// 2. Constructs a [`TextureAtlasLayout`] from the UV rects and sheet size.
    /// 3. Registers the layout as the labeled sub-asset
    ///    `"raster_font::texture_atlas_layout"` via
    ///    [`labeled_asset_scope`](LoadContext::labeled_asset_scope).
    /// 4. Returns [`BevyAtlasResources`] bundling all handles and offsets.
    fn build_resources(
        self,
        raw_font: RawFont<Handle<Image>>,
    ) -> Result<<Self::Backend as Backend>::Resources, Self::Error> {
        let size = raw_font.sheet.size;
        let image = raw_font.sheet.image;
        let (textures, offsets) = raw_font
            .glyphs
            .into_iter()
            .map(|UTokenProps { region, offset }| (region, offset))
            .unzip();
        let layout = TextureAtlasLayout { textures, size };
        let regions = self
            .load_context
            .labeled_asset_scope::<TextureAtlasLayout, Self::Error>(
                "raster_font::texture_atlas_layout".to_string(),
                |_scope| Ok(layout),
            )?;

        Ok(BevyAtlasResources {
            image,
            offsets,
            regions,
            sampler: FontSampler(self.settings.sampler.clone()),
        })
    }
}

/// A temporary view into a loaded [`BevyAtlasResources`], borrowing the
/// resolved [`TextureAtlasLayout`] from a [`Assets<TextureAtlasLayout>`] store.
///
/// Implements [`SpriteSheet`] so it can serve as the `Output` type of
/// [`FontResourceProvider`] and power glyph resolution in a
/// [`RasterFontCtx`](crate::backend::RasterFontCtx).
///
/// Created internally by the [`FontResourceProvider`] impl on
/// [`Assets<TextureAtlasLayout>`]; you do not construct this directly.
pub struct BevyAtlas<'f, 'r> {
    regions: &'r TextureAtlasLayout,
    offsets: &'f [IGlyphOffset],
}

impl SpriteSheet for BevyAtlas<'_, '_> {
    /// Resolves an [`AtlasIndex`] to its [`UTokenProps`] (UV rect + draw offset).
    ///
    /// Returns `None` if the index is out of bounds for either the regions or
    /// offsets slices, which would indicate a mismatch between the font that
    /// produced the index and the resources currently loaded.
    fn props(&self, index: &AtlasIndex) -> Option<UTokenProps> {
        let region = self.regions.textures.get(index.0)?;
        let offset = self.offsets.get(index.0)?;
        Some(UTokenProps {
            region: *region,
            offset: *offset,
        })
    }
}

/// An error returned when a bevy [`FontResourceProvider`] fails to upgrade
/// [`Backend::Resources`] into borrowed [`FontResourceProvider::Output`] due to missing asset
/// dependencies.
#[derive(Debug, Clone)]
pub struct TextureAtlasLayoutNotFoundError(pub Handle<TextureAtlasLayout>);

impl Error for TextureAtlasLayoutNotFoundError {}

impl std::fmt::Display for TextureAtlasLayoutNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            concat![
                "Failed to upgrade a Bevy raster font because its texture atlas layout was not found.",
                " This likely means the font asset was not loaded correctly, or",
                " there is a mismatch between the font and the atlas resources being borrowed."
            ]
        )
    }
}

/// [`Assets<TextureAtlasLayout>`] is can provide BevyBackend font resources by constructing a
/// [`BevyAtlas`].
impl FontResourceProvider for Assets<TextureAtlasLayout> {
    // type Input = &'r BevyAtlasResources;
    type Backend = BevyBackend;
    type Error = TextureAtlasLayoutNotFoundError;
    type Output<'f, 'r> = BevyAtlas<'f, 'r>;

    /// Attempts to construct a [`BevyAtlas`] by borrowing the font's atlas layout from
    /// [`Assets<TextureAtlasLayout>`].
    fn upgrade_font<'f, 'r>(
        &'r self,
        res_in: &'f <Self::Backend as Backend>::Resources,
    ) -> Result<Self::Output<'f, 'r>, Self::Error> {
        let regions = self
            .get(&res_in.regions)
            .ok_or_else(|| TextureAtlasLayoutNotFoundError(res_in.regions.clone()))?;
        let offsets = &res_in.offsets;
        Ok(BevyAtlas { regions, offsets })
    }
}

/// Determines how a raster font asset's texture atlas is sampled when rendered.
/// Typically nearest-neighbor for crisp pixel art output.
///
/// ## Default
///
/// ```
/// use bevy::image::ImageSampler;
/// use raster_font::prelude::*;
///
/// assert_eq!(FontSampler::default().0, ImageSampler::nearest());
/// ```
#[derive(TypePath, Component, Resource, Clone, Debug, Deserialize, Serialize)]
pub struct FontSampler(pub ImageSampler);

impl Default for FontSampler {
    #[inline]
    fn default() -> Self {
        FontSampler(ImageSampler::nearest())
    }
}
