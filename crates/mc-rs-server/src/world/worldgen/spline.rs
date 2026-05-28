//! Spline cubique vanilla (`net.minecraft...CubicSpline`), utilisée par les
//! density functions `minecraft:spline` (offset/factor/jaggedness du terrain).
//!
//! La coordonnée d'entrée est elle-même une density function ; la valeur d'un
//! point peut être une constante ou une spline imbriquée.

use std::sync::Arc;

use super::density::Df;

#[inline]
fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

pub enum SplineValue {
    Const(f64),
    Spline(Box<Spline>),
}

impl SplineValue {
    fn apply(&self, x: i32, y: i32, z: i32) -> f64 {
        match self {
            SplineValue::Const(c) => *c,
            SplineValue::Spline(s) => s.apply(x, y, z),
        }
    }
}

pub struct SplinePoint {
    pub location: f64,
    pub value: SplineValue,
    pub derivative: f64,
}

pub struct Spline {
    pub coordinate: Arc<Df>,
    pub points: Vec<SplinePoint>,
}

impl Spline {
    pub fn apply(&self, x: i32, y: i32, z: i32) -> f64 {
        let coord = self.coordinate.compute(x, y, z) as f32 as f64;
        let pts = &self.points;
        debug_assert!(!pts.is_empty());

        // Extrapolation linéaire hors bornes (comme vanilla).
        if coord <= pts[0].location {
            return pts[0].value.apply(x, y, z) + pts[0].derivative * (coord - pts[0].location);
        }
        let last = pts.len() - 1;
        if coord >= pts[last].location {
            return pts[last].value.apply(x, y, z)
                + pts[last].derivative * (coord - pts[last].location);
        }

        // Segment [k, k+1] contenant `coord`.
        let mut k = 0;
        while k < last && coord >= pts[k + 1].location {
            k += 1;
        }
        let p0 = &pts[k];
        let p1 = &pts[k + 1];
        let loc0 = p0.location;
        let loc1 = p1.location;
        let h = loc1 - loc0;
        let t = (coord - loc0) / h;

        let y0 = p0.value.apply(x, y, z);
        let y1 = p1.value.apply(x, y, z);
        let d0 = p0.derivative;
        let d1 = p1.derivative;

        // Interpolation d'Hermite vanilla.
        let a = d0 * h - (y1 - y0);
        let b = -d1 * h + (y1 - y0);
        lerp(t, y0, y1) + t * (1.0 - t) * lerp(t, a, b)
    }
}
