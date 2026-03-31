# ![Pixel Perfect](https://raw.githubusercontent.com/cuppachino/pixel_perfect/refs/tags/bevy_track_asset-v0.1.0/assets/brand/logo/pixel_perfect.svg)

[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/cuppachino/pixel_perfect#license)
[![CI](https://github.com/cuppachino/pixel_perfect/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/cuppachino/pixel_perfect/actions/workflows/ci.yml)
[![docs.rs](https://img.shields.io/docsrs/pixel_perfect?label=docs.rs)](https://docs.rs/pixel_perfect)
![Deps.rs Crate Dependencies (specific version)](https://img.shields.io/deps-rs/bevy/0.18.1?logo=bevy&label=Bevy&link=https%3A%2F%2Fdocs.rs%2Fbevy%2F0.18.1%2Fbevy%2F)

🚧👷

## Road to 0.1

1. [x] **[`bevy_track_asset`](https://crates.io/bevy_track_asset/latest/bevy_track_asset/)**:
       Utility for propagating asset reloads to dependent ECS data.

2. [ ] **Raster Font**: Custom bitmap-like fonts with new-lines, text-wrapping, ligatures, and
       other niceties.
   - [x] **[`raster_font`](https://crates.io/raster_font/latest/raster_font/)**: A simple, backend-agnostic format for defining glyphs and their layout.

   - **Text Renderers**: Strategies for rendering text to image targets.
     - [ ] **Composite**: Multiple glyphs batched into a single image target.
     - [ ] **Per-Glyph**: Each glyph rendered to its own image target.

   - **Text Targets**: Various render targets for different use cases, e.g. UI vs world
     text.
     - [ ] **Bevy UI**: `ImageNode`
     - [ ] **Bevy 2D**: `Sprite`
     - [ ] **Bevy 3D**: _todo_

- [ ] **Pixel Perfect Nine-Slice**: A 9-slice plugin that maintains its source pixel density,
      unlike Bevy's built-in `NineSlice` which stretches the source texture. Ideal for pixel
      art UIs.

- [ ] **Pixel Perfect Camera (2D\3D)**: Pixel-perfect camera plugins for 2d and 3d games.
  - [ ] **Responsive Letterboxing\* _Sans Frontières_**: Dynamically adjust a viewport to fit
        different screen sizes and aspect ratios.
    - [ ] **Resolution-based**: Configure a target resolution, and the camera will scale to fit the
          maximum viewport while maintaining the minimum aspect ratio, and optionally snap to integer scales to avoid artifacts.
    - [ ] **Aspect Ratio-based**: Configure a minimum aspect ratio, and the camera will scale to fit
          the maximum viewport while maintaining the target aspect ratio.

  - [ ] **Borders (optional)**: Spawn UI Nodes to fill letterboxed areas. Use regular bevy
        patterns, like Pixel Perfect Nine-Slice, to decorate them.

## Future

- **CRT Shaders**: Emulate the look of old CRT monitors, with options for scanlines, curvature, etc.
  Ideal for games that want to evoke a retro aesthetic.
  - [ ] **Fullscreen Post-processing**
  - [ ] **ExtendedMaterial**
  - [ ] **UiMaterial**

- **Gouraud Shader (3D)**: Gouraud-shaded material for "old-school" 3D graphics, with support
  for dynamic lighting. Ideal for low-poly models that want to look good without normal maps.
