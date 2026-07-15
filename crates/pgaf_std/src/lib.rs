use pgaf_sdk::registry::{
    FunctionDriverResource, Namespace, Registries, Registry, RegistryError,
    SiteGeneratorDriverResource,
};

mod function;
mod site;

pub fn init(namespace: &Namespace, registries: &mut Registries) -> Result<(), RegistryError> {
    register_sitegen_drivers(namespace, registries.regmut_sitegen_drivers())?;
    register_function_drivers(namespace, registries.regmut_function_drivers())?;
    Ok(())
}

fn register_sitegen_drivers(
    _namespace: &Namespace,
    _registry: &mut Registry<SiteGeneratorDriverResource>,
) -> Result<(), RegistryError> {
    #[cfg(feature = "gdal")]
    {
        let driver = crate::site::vector::VECTOR_DRIVER;
        registry.register(
            namespace,
            "vector",
            SiteGeneratorDriverResource(driver.clone().coerce_to_dynamic()),
        )?;
    }

    #[cfg(feature = "gdal")]
    {
        let driver = crate::site::raster::RASTER_DRIVER;
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
) -> Result<(), RegistryError> {
    use crate::function::*;

    registry.register(
        namespace,
        "greet",
        FunctionDriverResource(greet::GREET_DRIVER.clone()),
    )?;

    Ok(())
}
