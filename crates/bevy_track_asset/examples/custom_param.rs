use bevy::{
    ecs::{
        query::QueryIter,
        system::{SystemParam, lifetimeless::Write},
    },
    image::{ImageLoaderSettings, ImageSampler},
    log::{Level, LogPlugin},
    prelude::*,
};

use bevy_track_asset::prelude::*;

/// Depends on an `Image` asset, and will be marked as changed when the asset reloads.
#[derive(Component, Resource)]
struct UseImage(Handle<Image>);

/// Iterator item returned by the custom system param.
enum ImageDependency<'w> {
    QueryUsesImage(Mut<'w, UseImage>),
    GlobalUseImage(ResMut<'w, UseImage>),
}

impl AssetDependent<Image> for ImageDependency<'_> {
    #[inline]
    fn asset_id(&self) -> AssetId<Image> {
        match self {
            ImageDependency::QueryUsesImage(m) => m.0.id(),
            ImageDependency::GlobalUseImage(r) => r.0.id(),
        }
    }
}

// Required to mark items changed when the asset reloads.
impl TriggerChangeDetection for ImageDependency<'_> {
    #[inline]
    fn trigger_change(&mut self) {
        match self {
            ImageDependency::QueryUsesImage(m) => m.set_changed(),
            ImageDependency::GlobalUseImage(r) => r.set_changed(),
        }
    }
}

/// Example of a custom system param that marks resources and components as changed when their
/// associated asset is modified.
#[derive(SystemParam)]
struct UsingImage<'w, 's> {
    query_uses_image: Query<'w, 's, Write<UseImage>>,
    global_use_image: ResMut<'w, UseImage>,
}

impl<'w, 's> IntoIterator for UsingImage<'w, 's> {
    type Item = ImageDependency<'w>;
    type IntoIter = std::iter::Chain<
        std::iter::Once<ImageDependency<'w>>,
        std::iter::Map<
            QueryIter<'w, 's, &'static mut UseImage, ()>,
            fn(Mut<'w, UseImage>) -> ImageDependency<'w>,
        >,
    >;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        // Yield the resource first, then the query items. The order doesn't matter, but it could.
        std::iter::once(ImageDependency::GlobalUseImage(self.global_use_image)).chain(
            self.query_uses_image.into_iter().map(
                ImageDependency::QueryUsesImage
                    as fn(bevy::prelude::Mut<'w, UseImage>) -> ImageDependency<'w>,
            ),
        )
    }
}

fn main() -> AppExit {
    let mut app = App::new();

    app.add_plugins((
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
                    ",custom_param=trace"
                ]
                .to_string(),
                ..default()
            },
        ),
        #[cfg(feature = "bevy_app")]
        AssetTrackingPlugin::default(),
    ));

    #[cfg(not(feature = "bevy_app"))]
    app.configure_sets(
        PostUpdate,
        TrackAssetSystems::Watcher.before(TrackAssetSystems::Reload),
    );

    app.add_systems(Startup, load_image)
        .add_systems(Update, greeting_system)
        .add_systems(
            PostUpdate,
            (
                set_changed_on_asset_reload_system::<Image, UsingImage>(),
                handle_changed_image_system.in_set(TrackAssetSystems::Reload),
            ),
        )
        .run()
}

/// Loads an image asset and creates 2 dependents of it.
fn load_image(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    let image: Handle<Image> = asset_server.load_with_settings(
        "brand/logo/bevy_track_asset.png",
        |s: &mut ImageLoaderSettings| {
            s.sampler = ImageSampler::nearest();
        },
    );

    info!("Begin load: {image:?}");

    commands.spawn(UseImage(image.clone()));
    commands.insert_resource(UseImage(image));
}

/// Logs a greeting for each entity with a `UseImage` component that was added or changed.
fn greeting_system(query: Query<Entity, Added<UseImage>>) {
    for new_user in query {
        info!("Hello image user, {new_user}");
    }
}

// Systems can handle changes with standard bevy change detection patterns.
fn handle_changed_image_system(
    query: Query<(Entity, &UseImage), Changed<UseImage>>,
    assets: Res<Assets<Image>>,
    global: Res<UseImage>,
) {
    if global.is_changed() {
        match assets.get(&global.0) {
            None => {
                info!("Changed (unloaded): Res<UseImage>")
            }
            Some(_) => info!("Changed: Res<UseImage> changed with loaded asset!"),
        };
    }

    for (entity, image) in query {
        let Some(_image) = assets.get(&image.0) else {
            info!("Changed (unloaded): {entity}");
            continue;
        };
        info!("Changed: {entity} changed with loaded asset!");
    }
}
