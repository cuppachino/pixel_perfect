# ![Raster Font]

[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/cuppachino/pixel_perfect#license)
[![CI](https://github.com/cuppachino/pixel_perfect/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/cuppachino/pixel_perfect/actions/workflows/ci.yml)
[![docs.rs](https://img.shields.io/docsrs/raster_font?label=docs.rs)](https://docs.rs/raster_font)
![Deps.rs Crate Dependencies (specific version)](https://img.shields.io/deps-rs/bevy/0.18.1?logo=bevy&label=Bevy&link=https%3A%2F%2Fdocs.rs%2Fbevy%2F0.18.1%2Fbevy%2F)

Raster Font is a declarative format for authoring and using image-backed fonts.

---

You may be looking for:

- [Top-level docs](https://docs.rs/raster_font)
- [TOML schema](https://docs.rs/raster_font/latest/raster_font/meta)
- [Layout syntax](https://docs.rs/raster_font/latest/raster_font/layout)
- [Bevy integration](https://docs.rs/raster_font/latest/raster_font/#bevy-integration)
- [Examples](https://github.com/cuppachino/pixel_perfect/tree/main/crates/raster_font/examples)
- [Repository](https://github.com/cuppachino/pixel_perfect)

## Install

```ps1
cargo add raster_font
```

## Quick Start

Author a font in TOML:

```toml
name   = "Example Font"
image  = "font.png"
layout = "abc$(->|=>)$(:\))Helowrd!"

[pack]
size   = [8, 8]
region = { min = [0, 0], max = [8, 8] }
```

`valid` returns an iterator of leftmost-longest input matches against the font's glyphs:

```rust
use raster_font::{backend::prelude::*, tree::InputResolver};

// Fonts with owned resources:
fn draw<B: Backend<Resources: SpriteSheet>>(font: &RasterFont<B>) {
    for glyph in font.valid("Hello -> world :)") {
        // Draw glyph
    }
}


// Fonts with borrowed resources:
fn draw_with<B: Backend>(
    font: &RasterFont<B>,
    provider: &impl FontResourceProvider<Backend = B>,
) {
    // Upgrade the font a `RasterFontCtx`
    let Ok(font_ctx) = font.upgrade(provider) else {
        return;
    };

    for glyph in font_ctx.valid("Hello world!") {
        // Draw glyph
    };
}

```

---

### Bevy Integration

Load it with Bevy using the `bevy` feature:

```rust
use bevy::prelude::*;
use raster_font::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, RasterFontAssetLoaderPlugin))
        .add_systems(Startup, load_font)
        .run();
}

#[derive(Resource)]
struct MyFont(Handle<RasterFont>);

fn load_font(asset_server: Res<AssetServer>, mut commands: Commands) {
    let font: Handle<RasterFont> =
        asset_server.load("font.toml");

    commands.insert_resource(MyFont(font));
}

fn render_text(
    mut commands: Commands,
    font: Res<MyFont>,
    layouts: Res<Assets<TextureAtlasLayout>>,
) {
    let Some(font) = font_assets.get(font) else {
        warn!("Font {font:?} not loaded yet");
        return;
    };

    // The bevy backend follows the borrowed resources pattern.
    for glyph in font
        .upgrade(&*texture_atlas_layout_assets)
        .expect("Texture atlas must be loaded if font is loaded")
        .valid("Hello, Bevy world!")
        .map(Option::unwrap)
    {
        // Draw glyph using region and offset data...
    }
}
```

## Feature Flags

|       Feature       | Description                                          |
| :-----------------: | :--------------------------------------------------- |
|       `bevy`        | Enables Bevy asset loading and integration           |
| `font_sequence_map` | Enables direct sequence lookup via `RasterFont::get` |

## Backend Examples

- [`bevy_asset`]: Demonstrates using the 1st-party `BevyBackend` to load `RasterFont` assets in the
  [Bevy game engine](https://bevy.org/).
- [`macroquad`]: Minimal backend example for [Macroquad](https://macroquad.rs/)

## Docs

Full usage including custom backends, layout syntax, and advanced glyph extraction are exhaustively
documented at [docs.rs/raster_font](https://docs.rs/raster_font).

## Roadmap

- [ ] **BIDI**: Some form of unicode to support bidirectional text layout. (right-to-left scripts, and
      mixing of left-to-right and right-to-left text)
- [ ] **Contextual glyph substitution**: e.g. when `S` is followed by `T`, allow substitution of
      `S` instead of `ST` to enable kerning and ligatures without
      needing to define separate sequences and glyphs for every combination of characters.
- [ ] **More backends**: Raster Font only has one 1st-party backend (bevy), but the API is designed
      to be backend-agnostic, as long as they have some concept of an image.

[`bevy_asset`]: https://github.com/cuppachino/pixel_perfect/blob/raster_font-v0.1.0/crates/raster_font/examples/bevy_asset.rs
[`macroquad`]: https://github.com/cuppachino/pixel_perfect/blob/raster_font-v0.1.0/crates/raster_font/examples/macroquad.rs
[Raster Font]: https://raw.githubusercontent.com/cuppachino/pixel_perfect/refs/tags/raster_font-v0.1.0/assets/brand/logo/raster_font.svg
