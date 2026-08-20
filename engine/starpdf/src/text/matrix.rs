#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix2D {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Default for Matrix2D {
    fn default() -> Self {
        Self::identity()
    }
}

impl Matrix2D {
    pub const fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    pub const fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    pub const fn translation(tx: f64, ty: f64) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    pub const fn scaling(sx: f64, sy: f64) -> Self {
        Self {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 0.0,
            f: 0.0,
        }
    }

    pub fn rotation(rad: f64) -> Self {
        let (sin, cos) = rad.sin_cos();
        Self {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Multiplies matrix `self` by `rhs` (self * rhs in PDF coordinate transformation order).
    pub fn multiply(&self, rhs: &Self) -> Self {
        Self {
            a: self.a * rhs.a + self.b * rhs.c,
            b: self.a * rhs.b + self.b * rhs.d,
            c: self.c * rhs.a + self.d * rhs.c,
            d: self.c * rhs.b + self.d * rhs.d,
            e: self.e * rhs.a + self.f * rhs.c + rhs.e,
            f: self.e * rhs.b + self.f * rhs.d + rhs.f,
        }
    }

    /// Transforms a point (x, y) by this matrix.
    pub fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        let x_prime = self.a * x + self.c * y + self.e;
        let y_prime = self.b * x + self.d * y + self.f;
        (x_prime, y_prime)
    }

    /// Calculates rotation angle in radians from the horizontal axis.
    pub fn rotation_radians(&self) -> f64 {
        self.b.atan2(self.a)
    }

    /// Calculates rotation angle in degrees (0..360).
    pub fn rotation_degrees(&self) -> f64 {
        let deg = self.rotation_radians().to_degrees();
        if deg < 0.0 {
            deg + 360.0
        } else {
            deg
        }
    }

    /// Computes the scaling factor on the horizontal axis.
    pub fn scale_x(&self) -> f64 {
        (self.a * self.a + self.b * self.b).sqrt()
    }

    /// Computes the scaling factor on the vertical axis.
    pub fn scale_y(&self) -> f64 {
        (self.c * self.c + self.d * self.d).sqrt()
    }
}
