# ![Pixel Perfect](https://raw.githubusercontent.com/cuppachino/pixel_perfect/refs/heads/main/assets/brand/logo.svg)

[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/cuppachino/pixel_perfect#license)
[![CI](https://github.com/cuppachino/pixel_perfect/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/cuppachino/pixel_perfect/actions/workflows/ci.yml)
[![docs.rs](https://img.shields.io/docsrs/pixel_perfect?label=docs.rs)](https://docs.rs/pixel_perfect)
![Deps.rs Crate Dependencies (specific version)](https://img.shields.io/deps-rs/bevy/0.18.1?logo=bevy&label=Bevy&link=https%3A%2F%2Fdocs.rs%2Fbevy%2F0.18.1%2Fbevy%2F)

🚧👷

## Roadmap

- [x] **Bevy Track Asset**: Utility for propagating asset reloads to dependent ECS data.
  - [ ] **Raster Font**: Custom bitmap-like fonts with, with support for ligatures and other niceties.
    - [ ] **Raster Text Renderers**: Multiple "Raster Font" text renderers for different use cases.
  - [ ] **Pixel Perfect Nine-Slice**: A 9-slice plugin that maintains its source pixel density, unlike Bevy's built-in `NineSlice` which stretches the source texture. Ideal for pixel art UIs.
- [ ] **Pixel Perfect Camera 2d\3d**: Pixel-perfect camera plugins for 2d and 3d games. Proper "letterboxing sans borders" when the window's aspect ratio doesn't match the target aspect ratio, and optional pixel-snapping. Ideal for games that want to look crisp at any resolution and any aspect ratio.
- [ ] **Gouraud Material**: Gouraud-shaded material for "old-school" 3D graphics, with support for dynamic lighting. Ideal for low-poly models that want to look good without normal maps.
