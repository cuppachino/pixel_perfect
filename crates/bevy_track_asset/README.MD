# ![Bevy Track Asset](https://raw.githubusercontent.com/cuppachino/pixel_perfect/refs/heads/main/crates/bevy_track_asset/assets/brand/logo.svg)

[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/cuppachino/pixel_perfect#license)
[![CI](https://github.com/cuppachino/pixel_perfect/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/cuppachino/pixel_perfect/actions/workflows/ci.yml)
[![docs.rs](https://img.shields.io/docsrs/bevy_track_asset?label=docs.rs)](https://docs.rs/bevy_track_asset)
![Deps.rs Crate Dependencies (specific version)](https://img.shields.io/deps-rs/bevy/0.18.1?logo=bevy&label=Bevy&link=https%3A%2F%2Fdocs.rs%2Fbevy%2F0.18.1%2Fbevy%2F)

Utility for propagating asset reloads to dependent ECS data.

When an asset is modified or finishes loading, all components/resources that depend on it are
automatically marked as changed so systems can react using standard Bevy change detection.

## Install

```toml
[dependencies]
bevy_track_asset = "0.1"
```

## Quick Start

Configure the `TrackAssetSystems` system set in your app:

```rust
use bevy::prelude::*;
use bevy_track_asset::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            AssetTrackingPlugin::default(), // AssetTrackingPlugin::<PostUpdate>::new(),
        ));
}
```

Implement `AssetDependent` for your data:

```rust
impl AssetDependent<MyAsset> for MyComponent {
    fn asset_id(&self) -> AssetId<MyAsset> {
        self.handle.id()
    }
}
```

Then register tracking:

```rust
app.add_plugins(
    TrackAssetPlugin::<MyAsset, Query<&mut MyComponent>>::default()
);
```

React using normal change detection:

```rust
fn system(query: Query<&MyComponent, Changed<MyComponent>>) {
    // recompute derived data
}
```

## Docs

Full usage and advanced patterns, like custom system params, are exhaustively documented at
[docs.rs/bevy_track_asset](https://docs.rs/bevy_track_asset).

## Roadmap

- [ ] **Derive `AssetDependent`**: Macro for deriving `AssetDependent` implementations for
      structs with `Handle<T>` fields, reducing boilerplate and potential for errors.
