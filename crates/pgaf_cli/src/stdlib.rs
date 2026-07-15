use std::error::Error;

use pgaf_sdk::registry::{
    FunctionDriverResource, Namespace, Registries, Registry, SiteGeneratorDriverResource,
};

pub fn init(namespace: &Namespace, registries: &mut Registries) -> Result<(), Box<dyn Error>> {
    register_sitegen_drivers(namespace, registries.regmut_sitegen_drivers())?;
    register_function_drivers(namespace, registries.regmut_function_drivers())?;
    Ok(())
}

#[allow(unused_variables, unused_imports)]
fn register_sitegen_drivers(
    namespace: &Namespace,
    registry: &mut Registry<SiteGeneratorDriverResource>,
) -> Result<(), Box<dyn Error>> {
    use crate::sites::drivers::*;

    #[cfg(feature = "gdal")]
    {
        let driver = DRIVER_VECTOR;
        registry.register(
            namespace,
            "vector",
            SiteGeneratorDriverResource(driver.clone().coerce_to_dynamic()),
        )?;
    }

    #[cfg(feature = "gdal")]
    {
        let driver = DRIVER_RASTER;
        registry.register(
            namespace,
            "raster",
            SiteGeneratorDriverResource(driver.clone().coerce_to_dynamic()),
        )?;
    }

    Ok(())
}

fn register_function_drivers(
    namespace: &Namespace,
    registry: &mut Registry<FunctionDriverResource>,
) -> Result<(), Box<dyn Error>> {
    use crate::functions::*;

    registry.register(
        namespace,
        "greet",
        FunctionDriverResource(GREET_DRIVER.clone()),
    )?;

    Ok(())
}
