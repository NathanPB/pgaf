use pgaf_sdk::data::GeoDeg;
use pgaf_sdk::domain::{DomainGeneratorCreate, DomainGeneratorDriverTyped, ExecutionUnit, UnitId};
use serde::Deserialize;
use std::sync::LazyLock;
use validator::Validate;

/// A synthetic domain generator that yields a regular grid of [`ExecutionUnit`]s within a bounding box.
///
/// Grid points are placed at `lon = west + i * resolution`, `lat = south + j * resolution`
/// for `i in 0..n_cols`, `j in 0..n_rows`, where `n_cols = floor((east - west) / resolution)`,
/// `n_rows = floor((north - south) / resolution)`. IDs are sequential row-major integers.
#[derive(Deserialize, Validate, Clone, Debug)]
pub struct RectDomainGeneratorConfig {
    /// `[west, south, east, north]` in decimal degrees.
    pub bbox: [f64; 4],

    /// Grid spacing in decimal degrees. Must be positive.
    #[validate(range(min = 1e-10))]
    pub resolution: f64,
}

pub struct RectDomainGenerator {
    config: RectDomainGeneratorConfig,
    i: usize,
    j: usize,
    n_cols: usize,
    n_rows: usize,
}

impl RectDomainGenerator {
    pub fn new(config: RectDomainGeneratorConfig) -> Self {
        let [west, south, east, north] = config.bbox;
        let res = config.resolution;
        let n_cols = ((east - west) / res).floor() as usize;
        let n_rows = ((north - south) / res).floor() as usize;
        Self {
            config,
            i: 0,
            j: 0,
            n_cols,
            n_rows,
        }
    }
}

impl Iterator for RectDomainGenerator {
    type Item = ExecutionUnit;

    fn next(&mut self) -> Option<Self::Item> {
        if self.n_cols == 0 || self.j >= self.n_rows {
            return None;
        }

        let [west, south, ..] = self.config.bbox;
        let res = self.config.resolution;

        let lon = west + self.i as f64 * res;
        let lat = south + self.j as f64 * res;
        let id = (self.j * self.n_cols + self.i) as i64;

        self.i += 1;
        if self.i >= self.n_cols {
            self.i = 0;
            self.j += 1;
        }

        Some(ExecutionUnit {
            id: UnitId::from(id),
            lon: GeoDeg::from(lon),
            lat: GeoDeg::from(lat),
        })
    }
}

pub struct RectDriver;

impl DomainGeneratorCreate<RectDomainGeneratorConfig> for RectDriver {
    type Generator = RectDomainGenerator;

    fn create(
        config: RectDomainGeneratorConfig,
    ) -> Result<Self::Generator, Box<dyn std::error::Error>> {
        Ok(RectDomainGenerator::new(config))
    }
}

pub static RECT_DRIVER: LazyLock<pgaf_sdk::domain::Driver> = LazyLock::new(|| {
    DomainGeneratorDriverTyped::<RectDriver, RectDomainGeneratorConfig>::default()
        .coerce_to_dynamic()
});

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gen(bbox: [f64; 4], resolution: f64) -> RectDomainGenerator {
        RectDomainGenerator::new(RectDomainGeneratorConfig { bbox, resolution })
    }

    #[test]
    fn test_2x2_grid_count() {
        let units: Vec<_> = make_gen([0.0, 0.0, 2.0, 2.0], 1.0).collect();
        assert_eq!(units.len(), 4);
    }

    #[test]
    fn test_2x2_grid_coords() {
        let units: Vec<_> = make_gen([0.0, 0.0, 2.0, 2.0], 1.0).collect();

        assert_eq!(units[0].lon, GeoDeg::from(0.0));
        assert_eq!(units[0].lat, GeoDeg::from(0.0));

        assert_eq!(units[1].lon, GeoDeg::from(1.0));
        assert_eq!(units[1].lat, GeoDeg::from(0.0));

        assert_eq!(units[2].lon, GeoDeg::from(0.0));
        assert_eq!(units[2].lat, GeoDeg::from(1.0));

        assert_eq!(units[3].lon, GeoDeg::from(1.0));
        assert_eq!(units[3].lat, GeoDeg::from(1.0));
    }

    #[test]
    fn test_sequential_ids() {
        let units: Vec<_> = make_gen([0.0, 0.0, 3.0, 2.0], 1.0).collect();
        assert_eq!(units.len(), 6);
        for (idx, unit) in units.iter().enumerate() {
            assert_eq!(unit.id, UnitId::from(idx as i64));
        }
    }

    #[test]
    fn test_3x2_grid_count() {
        let units: Vec<_> = make_gen([10.0, 20.0, 13.0, 22.0], 1.0).collect();
        assert_eq!(units.len(), 6);
    }

    #[test]
    fn test_resolution_half_degree() {
        let units: Vec<_> = make_gen([0.0, 0.0, 1.0, 1.0], 0.5).collect();
        assert_eq!(units.len(), 4);
        assert_eq!(units[0].lon, GeoDeg::from(0.0));
        assert_eq!(units[1].lon, GeoDeg::from(0.5));
    }

    #[test]
    fn test_empty_when_zero_cols_or_rows() {
        let units: Vec<_> = make_gen([0.0, 0.0, 0.5, 2.0], 1.0).collect();
        assert_eq!(units.len(), 0);
    }
}
