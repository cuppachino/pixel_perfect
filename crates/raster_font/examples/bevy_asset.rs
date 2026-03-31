use bevy::{
    ecs::{
        query::QueryIter,
        system::{SystemParam, lifetimeless::Write},
    },
    image::ImageSampler,
    log::{Level, LogPlugin},
    prelude::*,
};
use bevy_track_asset::prelude::*;
use raster_font::prelude::*;

/// Some global font resource.
#[derive(Resource)]
struct GlobalRasterFont(Handle<RasterFont>);

/// Component that holds a handle to a `RasterFont` asset.
#[derive(Component)]
struct RasterText {
    font: Handle<RasterFont>,
    text: Name, // name has efficient change detection.
}

impl AssetDependent<RasterFont> for RasterText {
    #[inline]
    fn asset_id(&self) -> AssetId<RasterFont> {
        self.font.id()
    }
}

#[derive(SystemParam)]
#[allow(dead_code)] // not dead
struct UsingFont<'w, 's> {
    query_text: Query<'w, 's, &'static mut RasterText>,
    default_font: ResMut<'w, GlobalRasterFont>,
}

#[allow(dead_code)] // not dead
enum FontDependency<'w> {
    QueryText(Mut<'w, RasterText>),
    DefaultFont(ResMut<'w, GlobalRasterFont>),
}

impl<'w, 's> IntoIterator for UsingFont<'w, 's> {
    type Item = FontDependency<'w>;
    type IntoIter = std::iter::Chain<
        std::iter::Map<
            QueryIter<'w, 's, Write<RasterText>, ()>,
            fn(Mut<'w, RasterText>) -> FontDependency<'w>,
        >,
        std::iter::Once<FontDependency<'w>>,
    >;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.query_text
            .into_iter()
            .map(FontDependency::QueryText as fn(Mut<'w, RasterText>) -> FontDependency<'w>)
            .chain(std::iter::once(FontDependency::DefaultFont(
                self.default_font,
            )))
    }
}

/// This example demonstrates loading a [`RasterFont`] asset using Bevy's asset
/// system.
///
/// [Bevy Track Asset](bevy_track_asset) is used to register the asset as a dependency of
/// [`RasterText`] components. `RasterText` implements [`AssetDependent<RasterFont>`], so
/// it will be marked as changed whenever the underlying `RasterFont` asset is changed.
fn main() -> AppExit {
    App::new()
        .add_plugins((
            // Add default bevy plugins.
            DefaultPlugins.set(
                // This makes it easier to read the output of the example.
                LogPlugin {
                    level: Level::TRACE,
                    filter: concat![
                        "warn",
                        ",wgpu_hal=error",
                        ",bevy_winit::system=info",
                        ",bevy_track_asset=trace",
                        ",bevy_asset=trace"
                    ]
                    .to_string(),
                    ..default()
                },
            ),
            // Add the raster font asset loader plugin, configured to track `RasterText` components.
            RasterFontAssetLoaderPlugin,
            AssetTrackingPlugin::default(),
        ))
        .add_systems(Startup, load_font_system)
        .add_systems(
            PostUpdate,
            (
                set_changed_on_asset_reload_system::<RasterFont, Query<&mut RasterText>>(),
                print_changed_glyphs_system.in_set(TrackAssetSystems::Reload),
            ),
        )
        .run()
}

fn load_font_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font: Handle<RasterFont> = asset_server.load_with_settings(
        "fonts/m5x7/font.toml",
        |s: &mut RasterFontLoaderSettings| {
            s.sampler = ImageSampler::linear();
        },
    );
    info!("Begin load {font:?}");

    commands.insert_resource(GlobalRasterFont(font.clone()));
    commands.spawn(RasterText {
        font,
        text: "Hello, world!".into(),
    });
}

fn print_changed_glyphs_system(
    global_font: Res<GlobalRasterFont>,
    query_text: Query<&RasterText, Changed<RasterText>>,
    font_assets: Res<Assets<RasterFont>>,
    texture_atlas_layout_assets: Res<Assets<TextureAtlasLayout>>,
) {
    if global_font.is_changed() {
        if let Some(font) = font_assets.get(&global_font.0) {
            info!(
                "Global font {} changed!",
                font.name.as_deref().unwrap_or("<unnamed>")
            );
        } else {
            warn!("Global {:?} not loaded yet", global_font.0);
        }
    }

    for RasterText { font, text } in &query_text {
        let Some(font) = font_assets.get(font) else {
            warn!("Font {font:?} not loaded yet");
            continue;
        };

        info!(
            "Changed text '{text}' with font {}",
            font.name.as_deref().unwrap_or("<unnamed>")
        );
        for glyph in font
            .upgrade(&*texture_atlas_layout_assets)
            .unwrap()
            .valid(text.as_bytes())
            .map(Option::unwrap)
        {
            info!("  Glyph: {glyph:?}");
        }
    }
}
