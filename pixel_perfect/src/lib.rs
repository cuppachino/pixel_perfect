//! # ![Pixel Perfect](https://raw.githubusercontent.com/cuppachino/pixel_perfect/refs/heads/main/assets/brand/logo/pixel_perfect.svg)
//!
//! **Pixel Perfect** is a collection of features for making games with a pixel art aesthetic in the
//! [Bevy game engine](https://bevyengine.org/).
#![cfg_attr(
    feature = "bevy",
    doc = r#"
## Bevy Integration

Pixel Perfect is extremely modular and easy to add to a bevy project. Add all enabled features
to a bevy project by adding the [`PixelPerfectPlugins`] plugin group to your app.

### Example

```no_run
use bevy::prelude::*;
use pixel_perfect::prelude::*;

fn main() -> AppExit {
    App::new()
        .add_plugins((DefaultPlugins, PixelPerfectPlugins))
        // ...
        .run()
}
```

And that's it! All enabled features will be available for use.

```no_run
use bevy::prelude::*;
use pixel_perfect::prelude::*;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {"#
)]
#![cfg_attr(
    all(feature = "bevy", feature = "font"),
    doc = r#"    let font: Handle<RasterFont> = assets.load("fonts/m5x7/font.toml");"#
)]
// UI Example
#![cfg_attr(
    all(feature = "bevy_ui", feature = "composite_text"),
    doc = r#"

    commands.spawn(("#
)]
// if both 2d and ui are enabled, there needs to be an image node or node if this is the root of the hierarchy.
#![cfg_attr(
    all(feature = "bevy_ui", feature = "bevy_2d", feature = "composite_text",),
    doc = r#"
        ImageNode::default(),
"#
)]
#![cfg_attr(
    all(feature = "bevy_ui", feature = "composite_text",),
    doc = r#"
        font.text("Hello, UI world!"),
        CompositeText,
        FontScaling::Perfect,
    ));"#
)]
// 2D Example
#![cfg_attr(
    all(feature = "bevy_2d", feature = "composite_text"),
    doc = r#"

    commands.spawn((
        font.text("Hello, 2D world (composite)!"),
        CompositeText,
        FontScaling::None,
    ));"#
)]
#![cfg_attr(
    all(feature = "bevy_2d", feature = "font"),
    doc = r#"
    commands.spawn((
        font.text("Hello, 2D world (sprite per-glyph)!"),
        PerGlyphText,
        FontScaling::Perfect,
        Transform::from_xyz(0.0, -20.0, 1.0),
    ));"#
)]
#![cfg_attr(
    feature = "bevy",
    doc = r#"
}
```
"#
)]
//!
//! [`Sequence`]: crate::font::core::Sequence
//! [`Glyph`]: crate::font::core::Glyph

#[cfg(all(
    // We have text rendering features enabled...
    any(feature = "composite_text", feature = "per_glyph_text"), 
    // ...but no rendering output features are enabled.
    not(any(feature = "bevy_ui", feature = "bevy_2d"))
))]
const _: () = compile_error!(
    "Enable a rendering output (e.g. `bevy_ui` or `bevy_2d`) to use text rendering features."
);

/// Data-driven raster fonts for pixel art games.
#[cfg(feature = "font")]
pub use font;
#[cfg(feature = "font")]
pub mod text;

#[cfg(feature = "bevy_ui")]
pub mod ui;

pub mod prelude {
    #[allow(unused_imports)]
    #[cfg(feature = "font")]
    pub use crate::{font::prelude::*, text::prelude::*};
}
