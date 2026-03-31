use macroquad::prelude::*;
use raster_font::{
    backend::prelude::*,
    builder::prelude::*,
    core::{FontMeta, InputResolver, UGlyphSize},
};
use thiserror::Error;

/// A raster font backend for [`macroquad`].
#[derive(Clone, Copy, Debug)]
pub struct MacroquadBackend;

impl Backend for MacroquadBackend {
    type Resources = FontAtlas;
}

/// [`SpriteSheet`] resources for a [`RasterFont<MacroquadBackend>`].
#[derive(Debug)]
pub struct FontAtlas {
    pub texture: Texture2D,
    pub glyphs: Vec<UTokenProps>,
}

impl SpriteSheet for FontAtlas {
    /// Resolves an [`AtlasIndex`] to its [`UTokenProps`] (UV rect + draw offset).
    type Props = Option<UTokenProps>;

    /// Returns `None` if the index is out of bounds for the glyphs vector,
    /// which would indicate a mismatch between the font that produced the index and the resources
    /// held.
    fn props(&self, index: &raster_font::core::AtlasIndex) -> Self::Props {
        self.glyphs.get(**index).cloned()
    }
}

/// Final build command for a [`FontAtlasBuilder`] that produces [`RasterFont<MacroquadBackend>`].
///
/// In this example, the build command functions as settings for the font atlas builder, but it could
/// also be used to transfer ownership to other backend-specific resources, if you held a mutable
/// reference to them in the struct.
///
/// Note that the backend is not decided until calling [`build`](FontAtlasBuilder::build), so
/// you may clone the builder before then to use multiple backends that support the same font
/// resources if desired.
pub struct BuildMacroquadFont {
    filter: FilterMode,
}

impl BackendBuilder for BuildMacroquadFont {
    type Backend = MacroquadBackend;
    type Error = std::convert::Infallible;
    type Sheet = Texture2D;

    fn build_resources(
        self,
        raw_font: raster_font::builder::RawFont<Self::Sheet>,
    ) -> Result<<Self::Backend as Backend>::Resources, Self::Error> {
        // Apply settings
        raw_font.sheet.image.set_filter(self.filter);

        Ok(FontAtlas {
            texture: raw_font.sheet.image,
            glyphs: raw_font.glyphs,
        })
    }
}

#[derive(Debug, Error)]
enum Error {
    #[error("{0}")]
    FontBuilderError(#[from] FontBuilderError<<BuildMacroquadFont as BackendBuilder>::Error>),
    #[error("{0}")]
    MacroquadError(#[from] macroquad::Error),
    #[error("TOML deserialization error: {0}")]
    TomlError(#[from] toml::de::Error),
}

#[macroquad::main("Raster Font Example")]
async fn main() -> Result<(), Error> {
    macroquad::file::set_pc_assets_folder("assets");

    // Load and parse the font metadata
    let meta = macroquad::file::load_file("fonts/m5x7/font.toml").await?;
    let meta = toml::from_slice::<FontMeta>(&meta)?;

    // Load the font's glyph sheet texture and get its size.
    let sheet = {
        let image = macroquad::texture::load_texture("fonts/m5x7/font.png").await?;
        GlyphSheet {
            size: {
                let Vec2 { x, y } = image.size();
                UGlyphSize::new(x as u32, y as u32)
            },
            image,
        }
    };

    let font = FontAtlasBuilder::from(meta.layout.unique())
        .with_name(meta.name)
        .with_image(sheet)
        .populate_layout(&meta.layout)
        .custom_glyphs(meta.layout.custom())
        .map_err(FontBuilderError::UnknownSequence)?
        .build(BuildMacroquadFont {
            filter: FilterMode::Nearest,
        })?;

    println!(
        "Font loaded successfully: {}",
        font.name.as_deref().unwrap_or("<unnamed>"),
    );

    for glyph in font.valid("m5x7") {
        println!("{glyph:?}");
    }

    Ok(())
}
