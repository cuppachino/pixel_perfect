use std::error::Error;

use serde::{Deserialize, Serialize};

use bevy_app::prelude::*;
use bevy_asset::{AssetLoader, AssetPath, prelude::*};
use bevy_image::{Image, ImageSampler};
use bevy_reflect::prelude::*;

use crate::{
    backend::bevy_backend::{
        BuildBevyFont, loader::error::RasterFontAssetLoaderError, prelude::RasterFont,
    },
    builder::{FontAtlasBuilder, GlyphSheet, errors::FontBuilderError},
    meta::FontMeta,
};

/// Initialize the [`RasterFont`] asset type and [`RasterFontAssetLoader`] in a Bevy [`App`].
///
/// # Example
///
/// ```no_run
/// use bevy::{prelude::*, image::ImageSampler};
/// use raster_font::prelude::*;
///
/// fn main() -> AppExit {
///     App::new()
///         .add_plugins((DefaultPlugins, RasterFontAssetLoaderPlugin))
///         .add_systems(Startup, begin_load_font)
///         .run()
/// }
///
/// #[derive(Resource)]
/// struct MyFont {
///     handle: Handle<RasterFont>,
/// }
///
/// fn begin_load_font(asset_server: Res<AssetServer>, mut commands: Commands) {
///     let handle: Handle<RasterFont> = asset_server.load_with_settings(
///         "font.toml",
///         |s: &mut RasterFontLoaderSettings| {
///             s.sampler = ImageSampler::linear();
///         },
///     );
///     
///     commands.insert_resource(MyFont { handle });
/// }
/// ```
///
/// Note that assets are loaded asynchronously, so you must wait until the font is fully loaded
/// before accessing it. This has been omitted for brevity.
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use raster_font::prelude::*;
/// #
/// # #[derive(Resource)]
/// # struct MyFont {
/// #     handle: Handle<RasterFont>,
/// # }
/// #
/// fn use_font(
///     fonts: Res<Assets<RasterFont>>,
///     loaded_font: Res<MyFont>,
///     texture_atlas_layouts: Res<Assets<TextureAtlasLayout>>
/// ) {
///     if let Some(font) = fonts.get(&loaded_font.handle) {
///         let valid = font
///             .upgrade(&*texture_atlas_layouts)
///             .unwrap()
///             .valid("Hello world!")
///             .collect::<Vec<_>>();
///
///         info_once!("Glyphs for 'Hello world!': {valid:?}");
///     }
/// }
/// ```
#[derive(Default, TypePath)]
#[type_path = "raster_font::RasterFontAssetLoaderPlugin"]
pub struct RasterFontAssetLoaderPlugin;

impl Plugin for RasterFontAssetLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<RasterFont>()
            .init_asset_loader::<RasterFontAssetLoader>();
    }
}

/// Load settings for a [`RasterFont`] asset.
///
/// # Example
///
/// ```no_run
/// use bevy::{prelude::*, image::ImageSampler};
/// use raster_font::prelude::*;
///
/// fn load_font(asset_server: Res<AssetServer>) {
///     let font: Handle<RasterFont> = asset_server.load_with_settings(
///         "font.toml",
///         |s: &mut RasterFontLoaderSettings| {
///             s.sampler = ImageSampler::linear();
///         },
///     );
/// }
/// ```
///
/// ## Default Settings
///
/// The default settings use nearest-neighbor sampling, which is ideal for pixel art fonts.
///
/// ```
/// # use bevy::{prelude::*, image::ImageSampler};
/// # use raster_font::prelude::*;
/// #
/// assert_eq!(
///     RasterFontLoaderSettings::default(),
///     RasterFontLoaderSettings {
///         sampler: ImageSampler::nearest(),
///     }
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct RasterFontLoaderSettings {
    /// The image sampler to use for the raster font texture.
    pub sampler: ImageSampler,
}

impl Default for RasterFontLoaderSettings {
    fn default() -> Self {
        RasterFontLoaderSettings {
            sampler: ImageSampler::nearest(),
        }
    }
}

/// A Bevy asset loader for raster fonts, which are two-part assets consisting of a texture and
/// metadata defining the font layout and glyph properties.
#[derive(Default, TypePath)]
pub struct RasterFontAssetLoader;

impl AssetLoader for RasterFontAssetLoader {
    type Asset = RasterFont;
    type Settings = RasterFontLoaderSettings;
    type Error = RasterFontAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn bevy_asset::io::Reader,
        settings: &Self::Settings,
        load_context: &mut bevy_asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        // Read the meta file.
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let meta = toml::de::from_slice::<FontMeta>(&bytes)?;

        // Grab the texture and its size.
        let glyph_sheet = {
            let path = load_context
                .path()
                .parent()
                .map(|p| AssetPath::from_path_buf(p.path().join(&meta.image)))
                .unwrap_or(AssetPath::from_path_buf(meta.image));

            let texture_load = {
                let nested = load_context.loader().immediate().with_static_type();
                nested.load::<Image>(path).await?
            };

            GlyphSheet {
                size: texture_load.get().size(),
                image: load_context.add_loaded_labeled_asset("texture", texture_load),
            }
        };

        FontAtlasBuilder::from(meta.layout.unique())
            .with_name(meta.name)
            .with_image(glyph_sheet)
            .populate_layout(&meta.layout)
            .custom_glyphs(meta.layout.custom())?
            .build(BuildBevyFont {
                load_context,
                settings,
            })
            .map_err(RasterFontAssetLoaderError::FontBuilderError)
    }

    fn extensions(&self) -> &[&str] {
        &["font.toml", "toml"]
    }
}

pub mod error {

    use super::*;

    /// Possible errors that can be produced by [`RasterFontAssetLoader`]
    #[derive(Debug)]
    pub enum RasterFontAssetLoaderError {
        /// An [IO](std::io) Error
        Io(std::io::Error),
        /// A [TOML](toml) Error
        TomlError(toml::de::Error),
        /// A [Bevy Asset Loader](bevy_asset::LoadDirectError) Error
        LoadDirectError(bevy_asset::LoadDirectError),
        /// Errors produced by the raster font builder, including errors from constructing
        /// backend-specific resources.
        FontBuilderError(FontBuilderError<std::convert::Infallible>),
    }

    impl Error for RasterFontAssetLoaderError {}

    impl std::fmt::Display for RasterFontAssetLoaderError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                RasterFontAssetLoaderError::Io(e) => write!(f, "Could not load asset: {e}"),
                RasterFontAssetLoaderError::TomlError(e) => write!(f, "Could not parse TOML: {e}"),
                RasterFontAssetLoaderError::LoadDirectError(e) => {
                    write!(f, "Could not immediately load nested asset: {e}")
                }
                RasterFontAssetLoaderError::FontBuilderError(e) => {
                    write!(f, "Raster Font builder error: {e}")
                }
            }
        }
    }

    impl From<std::io::Error> for RasterFontAssetLoaderError {
        fn from(err: std::io::Error) -> Self {
            RasterFontAssetLoaderError::Io(err)
        }
    }
    impl From<toml::de::Error> for RasterFontAssetLoaderError {
        fn from(err: toml::de::Error) -> Self {
            RasterFontAssetLoaderError::TomlError(err)
        }
    }
    impl From<bevy_asset::LoadDirectError> for RasterFontAssetLoaderError {
        fn from(err: bevy_asset::LoadDirectError) -> Self {
            RasterFontAssetLoaderError::LoadDirectError(err)
        }
    }
    impl<E: Into<FontBuilderError<std::convert::Infallible>>> From<E> for RasterFontAssetLoaderError {
        fn from(err: E) -> Self {
            RasterFontAssetLoaderError::FontBuilderError(err.into())
        }
    }
}
