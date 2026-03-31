#[cfg(feature = "bevy_ui")]
use std::fmt::Debug;
use std::ops::Mul;

#[cfg(feature = "bevy")]
use bevy_ecs::prelude::*;
#[cfg(feature = "bevy")]
use bevy_reflect::prelude::*;

/// How to scale [`RasterText`] at different UI scales. Defaults to `Perfect`.
///
/// [`RasterText`] crate::text::RasterText
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(
    feature = "bevy_ui",
    derive(Component, Reflect),
    reflect(Component, Clone, Debug, Default)
)]
pub enum FontScaling {
    /// Ideal for pixel art, scales the image by the nearest integer factor of the original size.
    #[default]
    Perfect,
    /// Uses the exact scale factor, which may result in blurry images.
    Exact,
    /// No scaling applied.
    None,
}

pub trait FontScale<Value, Scalar = Value> {
    fn apply(&self, value: Value, scale: Scalar) -> Value;
}

impl<Value, Scalar> FontScale<Value, Scalar> for FontScaling
where
    Value: Mul<Scalar, Output = Value>,
{
    fn apply(&self, value: Value, scale: Scalar) -> Value {
        match self {
            FontScaling::None => value,
            FontScaling::Exact => value * scale,
            FontScaling::Perfect => {
                let scale = scale.round().max(Scalar::ONE);
                (value * scale).round()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_scale_glam() {
        use bevy::math::prelude::*;

        let value = Vec2::new(10.0, 5.0);
        let scale = 2.5;
        assert_eq!(FontScaling::None.apply(value, scale), Vec2::new(10.0, 5.0));
    }
}
