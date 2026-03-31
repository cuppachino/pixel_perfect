# ![Raster Font](https://raw.githubusercontent.com/cuppachino/pixel_perfect/refs/tags/raster_font-v0.1.0/assets/brand/logo/raster_font.svg)

[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/cuppachino/pixel_perfect#license)
[![CI](https://github.com/cuppachino/pixel_perfect/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/cuppachino/pixel_perfect/actions/workflows/ci.yml)
[![docs.rs](https://img.shields.io/docsrs/raster_font?label=docs.rs)](https://docs.rs/raster_font)
![Deps.rs Crate Dependencies (specific version)](https://img.shields.io/deps-rs/bevy/0.18.1?logo=bevy&label=Bevy&link=https%3A%2F%2Fdocs.rs%2Fbevy%2F0.18.1%2Fbevy%2F)

Data-driven raster fonts for pixel art games.

## Install

```ps1
cargo add raster_font
```

## Quick Start

Author a font in TOML:

```toml
name   = "Example Font"
image  = "font.png"
layout = "abc$(->|=>)"

[pack]
size   = [8, 8]
region = { min = [0, 0], max = [8, 8] }
```

Load it in Bevy using the `bevy` feature:

```rust
use bevy::prelude::*;
use raster_font::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, RasterFontAssetLoaderPlugin))
        .add_systems(Startup, load_font)
        .run();
}

fn load_font(asset_server: Res<AssetServer>) {
    let _font: Handle<RasterFont> = asset_server.load("font.toml");
}
```

Resolve input text into glyphs at runtime:

```rust
use raster_font::{backend::prelude::*, tree::InputResolver};

fn draw<B: Backend<Resources: SpriteSheet>>(font: &RasterFont<B>) {
    for glyph in font.valid("Hello -> world :)") {
        // Draw glyph
    }
}
```

Matching is leftmost-longest, so sequences like `->` naturally take precedence over their prefixes.

## Feature Flags

|       Feature       | Description                                          |
| :-----------------: | :--------------------------------------------------- |
|       `bevy`        | Enables Bevy asset loading and integration           |
| `font_sequence_map` | Enables direct sequence lookup via `RasterFont::get` |

## Docs

Full usage including custom backends, layout syntax, and advanced glyph extraction are exhaustively
documented at [docs.rs/raster_font](https://docs.rs/raster_font).

## Roadmap

- [ ] **BIDI**: Unicode code points and bidirectional text layout. (right-to-left scripts, and
      mixing of left-to-right and right-to-left text)
- [ ] **Contextual glyph substitution**: e.g. when `S` is followed by `T`, allow substitution of
      `S` instead of `ST` to enable kerning and ligatures without
      needing to define separate sequences and glyphs for every combination of characters.
- [ ] **More backends**: currently only Bevy is supported, but the API is designed
      to be backend-agnostic and should be implementable for any 2D rendering engine
      with a concept of sprite sheets.
