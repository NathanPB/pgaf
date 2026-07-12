use super::resources::*;
use super::{Namespace, Registry};
use std::error::Error;

pub fn init_itself(registries: &mut super::Registries) -> Result<Namespace, Box<dyn Error>> {
    let namespace = registries.claim_namespace("std")?;
    register_sitegen_drivers(&namespace, registries.regmut_sitegen_drivers())?;
    register_function_drivers(&namespace, registries.regmut_function_drivers())?;
    Ok(namespace)
}

#[allow(unused_variables, unused_imports)]
fn register_sitegen_drivers(
    namespace: &Namespace,
    registry: &mut Registry<SiteGeneratorDriverResource>,
) -> Result<(), Box<dyn Error>> {
    use crate::sites::drivers::*;

    #[cfg(feature = "gdal")]
    {
        registry.register(
            &namespace,
            "vector",
            SiteGeneratorDriverResource(DRIVER_VECTOR.clone().coerce_to_dynamic()),
        )?;
    }

    #[cfg(feature = "gdal")]
    {
        registry.register(
            &namespace,
            "raster",
            SiteGeneratorDriverResource(DRIVER_RASTER.clone().coerce_to_dynamic()),
        )?;
    }

    Ok(())
}

fn register_function_drivers(
    namespace: &Namespace,
    registry: &mut Registry<FunctionDriverResource>,
) -> Result<(), Box<dyn Error>> {
    use crate::functions::functions::*;

    registry.register(
        namespace,
        "greet",
        FunctionDriverResource(GREET_DRIVER.clone()),
    )?;

    Ok(())
}
