#![allow(unused_imports)]

use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use std::fmt::Debug;
use validator::Validate;

#[cfg(feature = "gdal")]
#[serde_inline_default]
#[derive(Validate, Deserialize, Clone, Debug)]
pub struct VectorSiteGeneratorConfig {
    #[validate(length(min = 1, message = "Vector file path cannot be empty"))]
    pub file: String,

    #[serde_inline_default("ID".to_string())]
    #[validate(length(min = 1, message = "Site ID key cannot be empty"))]
    pub site_id_key: String,
}

#[cfg(feature = "gdal")]
#[serde_inline_default]
#[derive(Validate, Deserialize, Clone, Debug)]
pub struct RasterSiteGeneratorConfig {
    pub file: String,

    #[serde_inline_default(0)]
    pub layer_index: usize,
}
