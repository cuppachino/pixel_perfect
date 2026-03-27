#![doc(
    html_logo_url = "https://raw.githubusercontent.com/cuppachino/pixel_perfect/refs/heads/main/crates/bevy_track_asset/assets/brand/logo.svg"
)]
//! # ![Bevy Track Asset](https://raw.githubusercontent.com/cuppachino/pixel_perfect/refs/heads/main/crates/bevy_track_asset/assets/brand/logo.svg)
//!
//! Utility for tracking asset reloads and propagating changes to dependent ECS data.
//!
//! When working with [assets](bevy_asset) in Bevy, it's common to cache derived data that depends
//! on an asset. For example, a resource that holds the output of a compute shader may need to be
//! re-computed whenever an asset it depends on changes.
//!
//! This crate provides a composable system, [`set_changed_on_asset_reload_system`], along with the
//! [`TrackAsset`], [`AssetDependent`], and [`TriggerChangeDetection`] traits, to automate this pattern: whenever
//! a tracked asset is modified or finishes loading, all dependent ECS items (components, resources,
//! etc.) are automatically marked as changed, so that downstream systems can react using Bevy's
//! standard change detection.
//!
//! # Feature flags
//!
//!| Default | Feature       | Description                                                                       |
//!| :-----: | ------------- | :-------------------------------------------------------------------------------- |
//!| Yes     | `bevy_app`    | Enables automatic scheduling with `AssetTrackingPlugin` and `TrackAssetPlugin`.   |
//!| Yes     | `bevy_log`    | Logs asset change events and warnings about missing plugins in debug builds.      |
//!
//! # Getting Started
//!
#![cfg_attr(
    feature = "bevy_app",
    doc = "\
Add an [`AssetTrackingPlugin`] to your Bevy app to automatically configure `TrackAssetSystems` in \
a schedule of your choice. Most users will want to handle changes in `PostUpdate`.

```no_run
use bevy::prelude::*;
use bevy_track_asset::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            AssetTrackingPlugin::<PostUpdate>::default(),
        ));
}
```"
)]
#![cfg_attr(
    not(feature = "bevy_app"),
    doc = "\
Configure the `TrackAssetSystems` system set in a schedule you would like to use for asset \
tracking systems, ensuring that the `Watcher` set runs before any systems that react to asset \
loads, which should be in the `Reload` set.

```no_run
use bevy::prelude::*;
use bevy_track_asset::prelude::*;

fn main() {
    App::new()
        .configure_sets(
            PostUpdate,
            TrackAssetSystems::Watcher.before(TrackAssetSystems::Reload),
        );
}
```"
)]
//!
//! # Declaring dependencies
//!
//! Implement [`AssetDependent`] for your component, resource, or custom system param item,
//! returning the [`AssetId`] of the asset it depends on:
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_track_asset::AssetDependent;
//!
//! #[derive(Asset, TypePath)]
//! struct MyShader { /* ... */ }
//!
//! #[derive(Component, Resource, Default)]
//! struct ComputedValue { /* ... */ }
//!
//! #[derive(Component, Resource)]
//! #[require(ComputedValue)]
//! struct ShaderUser {
//!     shader: Handle<MyShader>,
//!     // ...
//! }
//!
//! impl AssetDependent<MyShader> for ShaderUser {
//!     fn asset_id(&self) -> AssetId<MyShader> {
//!         self.shader.id()
//!     }
//! }
//! ```
//!
//! # Tracking the asset
//!
#![cfg_attr(
    feature = "bevy_app",
    doc = "\
Add a [`TrackAssetPlugin`] for your asset type and a system param that can extract the dependent \
items (e.g. `Query<&mut ShaderUser>`). This will add a `set_changed_on_asset_reload_system` to \
your app, which listens for asset events and marks dependent items as changed when their asset is \
reloaded.

```no_run
# use bevy::prelude::*;
# use bevy_track_asset::prelude::*;
#
# #[derive(Asset, TypePath)]
# struct MyShader;
#
# #[derive(Component, Resource)]
# struct ShaderUser {
#     shader: Handle<MyShader>,
#     // ...
# }
# impl AssetDependent<MyShader> for ShaderUser {
#     fn asset_id(&self) -> AssetId<MyShader> {
#         self.shader.id()
#     }
# }
#
# fn main() {
# let mut app = App::new();
app.add_plugins((
    TrackAssetPlugin::<MyShader, Query<&mut ShaderUser>>::default(),
    TrackAssetPlugin::<MyShader, TrackResMut<ShaderUser>>::default(),
));
# }
```"
)]
#![cfg_attr(
    not(feature = "bevy_app"),
    doc = "\
Then, add [`set_changed_on_asset_reload_system`] to your app, specifying the asset type and a
system param that provides access to the items you want to track (e.g. `Query<&mut ShaderUser>`).

```no_run
# use bevy::prelude::*;
# use bevy_track_asset::prelude::*;
#
# #[derive(Asset, TypePath)]
# struct MyShader;
# 
# #[derive(Component, Resource)]
# struct ShaderUser {
#     shader: Handle<MyShader>,
#     // ...
# }
#
# impl AssetDependent<MyShader> for ShaderUser {
#     fn asset_id(&self) -> AssetId<MyShader> {
#         self.shader.id()
#     }
# }
#
# fn main () {
# let mut app = App::new();
app.add_systems(PostUpdate,
    (
        set_changed_on_asset_reload_system::<MyShader, Query<Mut<ShaderUser>>>,
        set_changed_on_asset_reload_system::<MyShader, TrackResMut<ShaderUser>>,
    )
        .in_set(TrackAssetSystems::Watcher)
   );
# }
```"
)]
//!
//! # Reacting to changes
//!
//! Systems can handle changes with standard bevy change detection patterns.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_track_asset::prelude::*;
//!
//! # #[derive(Component, Default)]
//! # struct ComputedValue;
//! #
//! # #[derive(Asset, TypePath)]
//! # struct MyShader;
//! #
//! # impl MyShader {
//! #     fn compute(&self, user: &ShaderUser) -> ComputedValue {
//! #         ComputedValue
//! #     }
//! # }
//! #
//! # #[derive(Component, Resource)]
//! # #[require(ComputedValue)]
//! # struct ShaderUser {
//! #     shader: Handle<MyShader>,
//! #     // ...
//! # }
//! #
//! # impl AssetDependent<MyShader> for ShaderUser {
//! #     fn asset_id(&self) -> AssetId<MyShader> {
//! #         self.shader.id()
//! #     }
//! # }
//! #
//! fn main() -> AppExit {
//!     App::new()
//!         // ...other plugins
//!         .add_systems(PostUpdate, handle_changed_system.in_set(TrackAssetSystems::Reload))
//!         .run()
//! }
//!
//! fn handle_changed_system(
//!     mut query: Query<(&ShaderUser, &mut ComputedValue), Changed<ShaderUser>>,
//!     assets: Res<Assets<MyShader>>
//! ) {
//!     for (user, mut value) in &mut query {
//!         // Re-derive cached data from the reloaded asset...
//!         let shader = assets.get(&user.shader).unwrap();
//!         *value = shader.compute(user);
//!     }
//! }
//! ```
//!
//! # Custom `SystemParam`
//!
//! You can also use custom system params to track more complex dependencies, such as multiple
//! components or resources that depend on the same asset type. Just ensure that the `Item` type of
//! your system param implements `IntoIterator` over elements that are both `AssetDependent` and
//! `TriggerChangeDetection`, and the change-tracking system will handle the rest.
//!
//! The following example demonstrates a custom system parameter (`UsingImage`), that tracks
//! dependencies of `Image` (`UseImage` -- both a component and a resource), and marks them as changed
//! when the image behind their `Handle<Image>` is loaded or reloaded.
//!
//! ```rust
//! use bevy::{
//!     ecs::{
//!         query::QueryIter,
//!         system::{SystemParam, lifetimeless::Write},
//!     },
//!     prelude::*,
//! };
//! use bevy_track_asset::prelude::*;
//!
//! /// Depends on an `Image` asset, and will be marked as changed when the asset reloads.
//! #[derive(Component, Resource)]
//! struct UseImage(Handle<Image>);
//!
//! /// Iterator item returned by the custom system param.
//! enum ImageDependency<'w> {
//!     QueryUsesImage(Mut<'w, UseImage>),
//!     GlobalUseImage(ResMut<'w, UseImage>),
//! }
//!
//! impl AssetDependent<Image> for ImageDependency<'_> {
//!     #[inline]
//!     fn asset_id(&self) -> AssetId<Image> {
//!         match self {
//!             ImageDependency::QueryUsesImage(component) => component.0.id(),
//!             ImageDependency::GlobalUseImage(resource) => resource.0.id(),
//!         }
//!     }
//! }
//!
//! // Required to mark items changed when the asset reloads.
//! impl TriggerChangeDetection for ImageDependency<'_> {
//!     #[inline]
//!     fn trigger_change(&mut self) {
//!         match self {
//!             ImageDependency::QueryUsesImage(component) => component.set_changed(),
//!             ImageDependency::GlobalUseImage(resource) => resource.set_changed(),
//!         }
//!     }
//! }
//!
//! /// Example of a custom system param that
//! /// marks resources and components as changed
//! /// when their associated asset is modified.
//! #[derive(SystemParam)]
//! struct UsingImage<'w, 's> {
//!     query_uses_image: Query<'w, 's, Write<UseImage>>,
//!     global_use_image: ResMut<'w, UseImage>,
//! }
//!
//! impl<'w, 's> IntoIterator for UsingImage<'w, 's> {
//!     type Item = ImageDependency<'w>;
//!     type IntoIter = std::iter::Chain<
//!         std::iter::Map<
//!             QueryIter<'w, 's, Write<UseImage>, ()>,
//!             fn(Mut<'w, UseImage>) -> ImageDependency<'w>,
//!         >,
//!         std::iter::Once<ImageDependency<'w>>,
//!     >;
//!
//!     #[inline]
//!     fn into_iter(self) -> Self::IntoIter {
//!         self.query_uses_image
//!             .into_iter()
//!             .map(
//!                 ImageDependency::QueryUsesImage
//!                     as fn(bevy::prelude::Mut<'w, UseImage>) -> ImageDependency<'w>,
//!             )
//!             .chain(std::iter::once(ImageDependency::GlobalUseImage(
//!                 self.global_use_image,
//!             )))
//!     }
//! }
//! ```
//!
//! Check out the full example in [`examples/custom_param.rs`](examples/custom_param.rs) for a working
//! demonstration of this pattern.
#[cfg(feature = "bevy_app")]
use bevy_app::prelude::{App, Plugin, PostUpdate};
use bevy_asset::prelude::*;
#[cfg(feature = "bevy_app")]
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::{
    prelude::*,
    system::{StaticSystemParam, SystemParam, SystemParamItem},
};
#[cfg(feature = "bevy_log")]
use bevy_log::info;
#[cfg(all(feature = "bevy_log", feature = "bevy_app"))]
use bevy_log::warn;
use bevy_platform::collections::HashSet;

pub mod prelude {
    #[cfg(not(feature = "bevy_app"))]
    pub use crate::set_changed_on_asset_reload_system;
    pub use crate::{
        AssetDependent, TrackAsset, TrackAssetSystems, TrackResMut, TriggerChangeDetection,
    };
    #[cfg(feature = "bevy_app")]
    pub use crate::{AssetTrackingPlugin, TrackAssetPlugin};
}

/// Atomic that tracks if *any* [`AssetTrackingPlugin`]s have been added, to conditionally warn
/// about missing plugins in `TrackAssetPlugin::finish()`.
#[cfg(all(debug_assertions, feature = "bevy_app", feature = "bevy_log"))]
static IS_ASSET_TRACKING_PLUGIN_ADDED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Plugin that configures [`TrackAssetSystems`] in a schedule of your choice.
///
/// Most users will want to configure this in `PostUpdate` to be compatible with [`TrackAssetPlugin`].
#[cfg(feature = "bevy_app")]
#[derive(Default)]
pub struct AssetTrackingPlugin<S: ScheduleLabel + Default = PostUpdate> {
    _schedule: std::marker::PhantomData<S>,
}

#[cfg(feature = "bevy_app")]
impl<S: ScheduleLabel + Default> Plugin for AssetTrackingPlugin<S> {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            S::default(),
            TrackAssetSystems::Watcher.before(TrackAssetSystems::Reload),
        );

        #[cfg(all(debug_assertions, feature = "bevy_log"))]
        IS_ASSET_TRACKING_PLUGIN_ADDED.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// System sets for asset tracking systems. With the `bevy_app` feature enabled, these are
/// automatically configured by `AssetTrackingPlugin`. Without `bevy_app`, you need to configure
/// these yourself.
#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrackAssetSystems {
    /// Asset tracking systems that watch for asset reload events and trigger change detection on
    /// dependent items.
    ///
    /// These should run before any systems that react to asset reloads, which should be in the
    /// `Reload` set.
    Watcher,
    /// SystemSet for systems that react to asset reloads, which should run after `Watcher`.
    ///
    /// This is just a marker set; you can put any systems that need to react to asset reloads in
    /// this set, and configure it in your app as needed.
    Reload,
}

/// A marker trait for [`SystemParam`]s whose data depends on an asset and can trigger change
/// detection when the asset is changed.
///
/// This trait is the primary bound required by [`set_changed_on_asset_reload_system`]. It is automatically
/// implemented for any [`SystemParam`] that implements [`IntoIterator`] over elements that are
/// [`AssetDependent`] and [`TriggerChangeDetection`]. You do not need to implement this trait manually.
///
/// ## Supported param types
///
/// Any of the following work out of the box:
///
/// - `Query<&mut T>` where `T: AssetDependent`
/// - `TrackResMut<T>` where `T: AssetDependent + Resource`. This is a wrapper around `ResMut<T>` that implements `TrackAsset`.
/// - Custom [`SystemParam`]s whose `Item` satisfies the bounds
///
/// ## Example
///
/// ```rust
/// use bevy::{prelude::*, ecs::system::SystemParamItem};
/// use bevy_track_asset::prelude::*;
///
/// #[derive(Asset, TypePath)]
/// struct MyAsset;
///
/// #[derive(Component, Resource)]
/// struct Loaded(Handle<MyAsset>);
///
/// impl AssetDependent<MyAsset> for Loaded {
///     fn asset_id(&self) -> AssetId<MyAsset> {
///         self.0.id()
///     }
/// }
///
/// fn assert_tracks<A: Asset, S: TrackAsset<A>>()
///     where for<'w, 's> SystemParamItem<'w, 's, S>: IntoIterator<Item: AssetDependent<A> + TriggerChangeDetection>
/// {
/// }
/// assert_tracks::<MyAsset, Query<&mut Loaded>>();
/// assert_tracks::<MyAsset, TrackResMut<Loaded>>();
/// ```
pub trait TrackAsset<A: Asset>: SystemParam + Send + Sync + 'static
where
    for<'w, 's> SystemParamItem<'w, 's, Self>:
        IntoIterator<Item: AssetDependent<A> + TriggerChangeDetection>,
{
}
impl<T: SystemParam + Send + Sync + 'static, A: Asset> TrackAsset<A> for T where
    for<'w, 's> SystemParamItem<'w, 's, Self>:
        IntoIterator<Item: AssetDependent<A> + TriggerChangeDetection>
{
}

/// A wrapper for `ResMut<T>` that implements [`TrackAsset`], for tracking assets in resources.
///
/// ## Why?
///
/// Rust's future-proof orphan rules prevent us from implementing `TrackAsset` directly for
#[cfg_attr(
    not(feature = "bevy_app"),
    doc = "\r\
`ResMut<T>`, but you can use this wrapper type with [`set_changed_on_asset_reload_system`] to \
track asset dependencies in resources."
)]
#[cfg_attr(
    feature = "bevy_app",
    doc = "\r\
`ResMut<T>`, but you can use this wrapper type with [`TrackAssetPlugin`] to track \
asset dependencies in resources."
)]
#[derive(SystemParam)]
pub struct TrackResMut<'w, T: Resource> {
    /// The wrapped `ResMut<T>` that provides access to the resource you want to track.
    res: ResMut<'w, T>,
}

impl<'w, T: Resource> IntoIterator for TrackResMut<'w, T> {
    type Item = ResMut<'w, T>;
    type IntoIter = std::iter::Once<ResMut<'w, T>>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.res)
    }
}

/// Allows an ECS item to be flagged as needing to be re-derived from its associated asset.
///
/// This trait is called by [`set_changed_on_asset_reload_system`] when it detects that an asset
/// dependency has been modified or (re)loaded. It should trigger change detection for the item, so
/// downstream systems using [`Changed<T>`] can react to it using Bevy's standard change
/// detection patterns.
///
/// # Provided implementations
///
/// A blanket implementation is provided for all [`DetectChangesMut`] types (including [`Mut<T>`]
/// and [`ResMut<T>`]), which calls [`DetectChangesMut::set_changed`]. This covers most use cases
/// without any manual implementation.
///
/// You only need to implement this trait yourself if you are using a custom [`SystemParam`] whose
/// `Item` type cannot use `set_changed` directly — for example, a wrapper type that owns multiple
/// pieces of mutable data and needs custom logic to determine which ones to mark as changed when
/// an asset reloads, or a param that updates multiple types of data that depend on the same asset
/// at once.
pub trait TriggerChangeDetection {
    /// Marks this item as changed, so that systems using [`Changed<T>`] will react to it.
    #[doc(alias = "set_changed")]
    fn trigger_change(&mut self);
}

/// Blanket implementation for mutable ECS data implementing [`DetectChangesMut`].
///
/// This covers [`Mut<T>`] and [`ResMut<T>`].
impl<T: DetectChangesMut> TriggerChangeDetection for T {
    #[inline]
    fn trigger_change(&mut self) {
        self.set_changed();
    }
}

/// Declares that an ECS item depends on a specific asset, and can provide its [`AssetId`].
///
/// Implement this trait on your component, resource, or custom [`SystemParam`] so
/// [`set_changed_on_asset_reload_system`] can extract its asset dependency and trigger Bevy's
/// change detection when the asset is modified or reloaded.
///
/// # Provided implementations
///
/// Blanket implementations are provided for [`Mut<T>`] and [`ResMut<T>`] where `T: AssetDependent`,
/// so you only need to implement this on your underlying data type, not on Bevy's wrapper types.
///
/// See [`TrackResMut`] for information about tracking resources, specifically.
///
/// See [module-level docs](crate) for examples of implementing this trait and using it with the
/// tracking system.
pub trait AssetDependent<A: Asset> {
    /// Returns the ID of the asset this item depends on.
    fn asset_id(&self) -> AssetId<A>;
}

/// Blanket implementation of `AssetDependent` for bevy's `Mut` containers.
impl<A: Asset, T: AssetDependent<A>> AssetDependent<A> for Mut<'_, T> {
    #[inline]
    fn asset_id(&self) -> AssetId<A> {
        self.as_ref().asset_id()
    }
}

/// Blanket implementation of `AssetDependent` for bevy's `ResMut` containers.
impl<A: Asset, T: Resource + AssetDependent<A>> AssetDependent<A> for ResMut<'_, T> {
    #[inline]
    fn asset_id(&self) -> AssetId<A> {
        self.as_ref().asset_id()
    }
}

/// Convenience plugin for tracking asset changes in the [`TrackAssetSystems::Watcher`] system set
/// during [`PostUpdate`].
///
/// Alternatively, you can add `set_changed_on_asset_reload_system` directly. This plugin simply
/// guarantees that a watcher system is added before [`TrackAssetSystems::Reload`], but you can
/// configure this manually if you prefer.
///
/// See [`set_changed_on_asset_reload_system`] for how tracking works
#[cfg(feature = "bevy_app")]
pub struct TrackAssetPlugin<A: Asset, T: TrackAsset<A>>
where
    for<'w, 's> SystemParamItem<'w, 's, T>:
        IntoIterator<Item: AssetDependent<A> + TriggerChangeDetection>,
{
    _asset: std::marker::PhantomData<A>,
    _param: std::marker::PhantomData<T>,
}

#[cfg(feature = "bevy_app")]
impl<A: Asset, T: TrackAsset<A>> Default for TrackAssetPlugin<A, T>
where
    for<'w, 's> SystemParamItem<'w, 's, T>:
        IntoIterator<Item: AssetDependent<A> + TriggerChangeDetection>,
{
    fn default() -> Self {
        Self {
            _asset: std::marker::PhantomData,
            _param: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "bevy_app")]
impl<A: Asset, T: TrackAsset<A>> Plugin for TrackAssetPlugin<A, T>
where
    for<'w, 's> SystemParamItem<'w, 's, T>:
        IntoIterator<Item: AssetDependent<A> + TriggerChangeDetection>,
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            set_changed_on_asset_reload_system::<A, T>.in_set(TrackAssetSystems::Watcher),
        );
    }

    /// Warn if the [`AssetTrackingPlugin`] was not added.
    #[cfg(all(debug_assertions, feature = "bevy_log"))]
    fn finish(&self, _app: &mut App) {
        if !IS_ASSET_TRACKING_PLUGIN_ADDED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            warn!(
                "No AssetTrackingPlugin detected. Expect random system order and missed asset events"
            );
        }
    }
}

/// A system that marks ECS items as changed when the asset they depend on is modified or reloaded.
///
/// Add this system to your app to automatically propagate asset reload events to dependent
/// components or resources, enabling Bevy's standard change detection to pick them up.
#[cfg_attr(
    feature = "bevy_app",
    doc = "
# Scheduling

Prefer using [`TrackAssetPlugin`] instead of adding this system manually. This guarantees that the \
system is added in [`TrackAssetSystems::Watcher`] and makes it easier to schedule other systems \
after it by scheduling them in `TrackAssetSystems::Reload`."
)]
#[cfg_attr(
    not(feature = "bevy_app"),
    doc = "
# Scheduling

The system should be added to the [`TrackAssetSystems::Watcher`] system set to ensure it is scheduled
before any systems that react to asset reloads, which should be in [`TrackAssetSystems::Reload`]."
)]
///
/// # Description
///
/// The system reads [`AssetEvent`]s for asset type `A`, and for each `Modified` or
/// `LoadedWithDependencies` event, marks all items in the provided [`TrackAsset`]
/// param whose [`AssetDependent::asset_id`] matches the reloaded asset as changed via
/// [`TriggerChangeDetection::trigger_change`].
///
/// Internally, reloaded asset IDs are buffered into a [`Local<HashSet>`] before notifying param
/// items to avoid redundant calls to `trigger_change`.
///
/// # Type parameters
///
/// - `A`: The [`Asset`] type to watch for changes.
/// - `S`: A [`TrackAsset`] system param wrapping the ECS items to trigger change detection on (e.g.
///   `Query<&mut MyComponent>`, `TrackResMut<MyResource>`).
///
/// # Example
///
/// ```
/// use bevy::prelude::*;
/// use bevy_track_asset::{AssetDependent, set_changed_on_asset_reload_system, TrackAssetSystems};
///
/// #[derive(Asset, TypePath)] struct MyAsset;
///
/// #[derive(Component)] struct MyComponent { handle: Handle<MyAsset> }
/// impl AssetDependent<MyAsset> for MyComponent {
///     fn asset_id(&self) -> AssetId<MyAsset> { self.handle.id() }
/// }
///
/// // Register in your app:
///
/// fn main() {
///     let mut app = App::new();
///     // ...other plugins and systems...
///     app.add_systems(PostUpdate,
///         set_changed_on_asset_reload_system::<MyAsset, Query<&mut MyComponent>>
///             .in_set(TrackAssetSystems::Watcher)
///     );
/// }
/// ```
///
/// # Notes
///
/// - AssetEvents: `Added`, `Removed`, and `Unused` are intentionally
///   ignored. Only [`Modified`](AssetEvent::Modified) and
///   [`LoadedWithDependencies`](AssetEvent::LoadedWithDependencies) trigger change detection.
///   See [`extract_asset_id`] for the filtering logic.
/// - With the `bevy_log` feature enabled, a log message is emitted at `info` level each time an
///   asset reload is detected.
pub fn set_changed_on_asset_reload_system<A: Asset, S: TrackAsset<A>>(
    mut messages: MessageReader<AssetEvent<A>>,
    mut cache: Local<HashSet<AssetId<A>>>,
    param: StaticSystemParam<S>,
) where
    for<'w, 's> SystemParamItem<'w, 's, S>:
        IntoIterator<Item: AssetDependent<A> + TriggerChangeDetection>,
{
    for id in messages.read().filter_map(extract_asset_id) {
        cache.insert(id);
    }

    for mut item in param.into_inner() {
        let id = item.asset_id();
        if cache.contains(&id) {
            #[cfg(feature = "bevy_log")]
            info!(
                "{} {id} reloaded. Marking dependent {} as changed",
                std::any::type_name::<A>(),
                std::any::type_name::<S>()
            );

            item.trigger_change();
        }
    }

    cache.clear();
}

/// Extracts the [`AssetId`] from an [`AssetEvent`] if it represents a meaningful change.
///
/// Returns `Some(id)` for:
/// - [`AssetEvent::Modified`] — the asset's content was changed.
/// - [`AssetEvent::LoadedWithDependencies`] — the asset and all its dependencies finished loading.
///
/// Returns `None` for [`AssetEvent::Added`], [`AssetEvent::Removed`], and [`AssetEvent::Unused`],
/// as these do not indicate that previously-derived data needs to be invalidated.
///
/// This function is used internally by [`set_changed_on_asset_reload_system`] but is public for use in
/// custom change-tracking systems that need the same filtering logic.
#[inline]
pub fn extract_asset_id<A: Asset>(event: &AssetEvent<A>) -> Option<AssetId<A>> {
    match event {
        &AssetEvent::Modified { id } | &AssetEvent::LoadedWithDependencies { id } => Some(id),
        AssetEvent::Added { .. } | AssetEvent::Removed { .. } | AssetEvent::Unused { .. } => None,
    }
}
