#![allow(unused_imports)]

use super::{config::*, gen::*, SiteGeneratorDriver};
use std::sync::{Arc, LazyLock};

#[cfg(feature = "gdal")]
pub const DRIVER_VECTOR: LazyLock<
    SiteGeneratorDriver<VectorSiteGenerator, VectorSiteGeneratorConfig>,
> = LazyLock::new(|| SiteGeneratorDriver {
    create: Arc::new(|c: VectorSiteGeneratorConfig| {
        VectorSiteGenerator::new(c.file.as_str(), c.site_id_key)
    }),
    config_deserializer: Arc::new(serde_json::from_value),
});

#[cfg(feature = "gdal")]
pub const DRIVER_RASTER: LazyLock<
    SiteGeneratorDriver<RasterSiteGenerator, RasterSiteGeneratorConfig>,
> = LazyLock::new(|| SiteGeneratorDriver {
    create: Arc::new(|c: RasterSiteGeneratorConfig| {
        RasterSiteGenerator::new(c.file.as_str(), c.layer_index)
    }),
    config_deserializer: Arc::new(serde_json::from_value),
});
