use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};
use bevy_track_asset::{AssetTrackingPlugin, prelude::*};

use bevy_log::{Level, LogPlugin};

#[derive(Component, Resource)]
struct UseImage(Handle<Image>);

impl AssetDependent<Image> for UseImage {
    #[inline]
    fn asset_id(&self) -> AssetId<Image> {
        self.0.id()
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
                    ",auto_param=trace"
                ]
                .to_string(),
                ..default()
            },
        ),
        // With the `bevy_app` feature enabled, the `AssetTrackingPlugin` configures
        // the `TrackAssetSystems` system set in a schedule of your choice.
        // Default is `PostUpdate`.
        #[cfg(feature = "bevy_app")]
        AssetTrackingPlugin::default(),
    ));

    // If not using the `bevy_app` feature, you should manually configure `TrackAssetSystems`
    // for the schedule you want to run the asset tracking systems in.
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
                // Trigger bevy's change detection for items that depend on the reloaded asset.
                set_changed_on_asset_reload_system::<Image, Query<&mut UseImage>>(),
                set_changed_on_asset_reload_system::<Image, TrackResMut<UseImage>>(),
                // Handle changes
                handle_changed_image_system.in_set(TrackAssetSystems::Reload),
            ),
        );

    app.run()
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
