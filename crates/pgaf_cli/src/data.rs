const GEO_DEG_PRECISION: f64 = 100_000.0;

/// Type that represents a latitude or longitude in degrees. It holds coordinates with a fixed precision of up to 5 decimal places.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct GeoDeg(f32);

impl GeoDeg {
    /// Returns the value of the GeoDeg as f64.
    #[allow(dead_code)] // This is part of the public API, so it's not dead code.
    pub fn as_f64(self) -> f64 {
        self.0 as f64
    }

    /// Returns the value of the GeoDeg as f32.
    #[allow(dead_code)] // This is part of the public API, so it's not dead code.
    pub fn as_f32(self) -> f32 {
        self.0
    }

    /// Formats the latitude value as a string with a specified number of decimal places.
    ///
    /// - Positive values are suffixed with `"N"` (North).
    /// - Negative values are suffixed with `"S"` (South).
    /// - The decimal point is replaced with an underscore (`_`) for file-safe formatting.
    ///
    /// # Arguments
    ///
    /// * `places` - The number of decimal places to format the coordinate to.
    ///
    /// # Returns
    ///
    /// A formatted string representing the latitude coordinate.
    ///
    /// # Example
    ///
    /// ```
    /// let lat = GeoDeg::from(-12.3456);
    /// assert_eq!(lat.ns(4), "12_3456S");
    /// ```
    pub fn ns(&self, places: usize) -> String {
        format!(
            "{:.2$}{}",
            self.0.abs(),
            if self.0 >= 0.0 { "N" } else { "S" },
            places,
        )
        .replace(".", "_")
    }

    /// Formats the longitude value as a string with a specified number of decimal places.
    ///
    /// - Positive values are suffixed with `"E"` (East).
    /// - Negative values are suffixed with `"W"` (West).
    /// - The decimal point is replaced with an underscore (`_`) for file-safe formatting.
    ///
    /// # Arguments
    ///
    /// * `places` - The number of decimal places to format the coordinate to.
    ///
    /// # Returns
    ///
    /// A formatted string representing the longitude coordinate.
    ///
    /// # Example
    ///
    /// ```
    /// let lng = GeoDeg::from(78.9101);
    /// assert_eq!(lng.ew(3), "78_910E");
    /// ```
    pub fn ew(&self, places: usize) -> String {
        format!(
            "{:.2$}{}",
            self.0.abs(),
            if self.0 >= 0.0 { "E" } else { "W" },
            places,
        )
        .replace(".", "_")
    }
}

impl From<f64> for GeoDeg {
    /// Creates a new GeoDeg from an f64.
    fn from(value: f64) -> Self {
        Self((value * GEO_DEG_PRECISION).round() as f32 / GEO_DEG_PRECISION as f32)
    }
}

impl From<f32> for GeoDeg {
    /// Creates a new GeoDeg from an f32.
    fn from(value: f32) -> Self {
        Self((value * GEO_DEG_PRECISION as f32).round() / GEO_DEG_PRECISION as f32)
    }
}

impl std::ops::Add for GeoDeg {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::from(self.0 + other.0)
    }
}

impl std::ops::Sub for GeoDeg {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self::from(self.0 - other.0)
    }
}

impl std::ops::Mul<f32> for GeoDeg {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self::from(self.0 * scalar)
    }
}

impl std::ops::Div<f32> for GeoDeg {
    type Output = Self;
    fn div(self, scalar: f32) -> Self {
        Self::from(self.0 / scalar)
    }
}

impl std::fmt::Display for GeoDeg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:.5}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ns_we() {
        assert_eq!(GeoDeg::from(-1.0).ns(2), "1_00S");
        assert_eq!(GeoDeg::from(-1.0).ew(2), "1_00W");
        assert_eq!(GeoDeg::from(0.0).ns(2), "0_00N");
        assert_eq!(GeoDeg::from(0.0).ew(2), "0_00E");
        assert_eq!(GeoDeg::from(1.0).ns(2), "1_00N");
        assert_eq!(GeoDeg::from(1.0).ew(2), "1_00E");
    }
}
