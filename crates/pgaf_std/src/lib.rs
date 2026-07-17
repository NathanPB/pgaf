use pgaf_sdk::registry::{
    DomainGeneratorDriverResource, FunctionDriverResource, Namespace,
    PipelineStepTypeDriverResource, Registries, Registry, RegistryError,
};

mod domain;
mod function;
mod pipeline;

pub fn init(namespace: &Namespace, registries: &mut Registries) -> Result<(), RegistryError> {
    register_domaingen_drivers(namespace, registries.regmut_domaingen_drivers())?;
    register_function_drivers(namespace, registries.regmut_function_drivers())?;
    register_pipelinestep_drivers(namespace, registries.regmut_pipelinestep_drivers())?;
    Ok(())
}

#[allow(unused_variables)]
fn register_domaingen_drivers(
    namespace: &Namespace,
    registry: &mut Registry<DomainGeneratorDriverResource>,
) -> Result<(), RegistryError> {
    #[cfg(feature = "gdal")]
    {
        let driver = crate::domain::vector::VECTOR_DRIVER;
        registry.register(
            namespace,
            "vector",
            DomainGeneratorDriverResource(driver.clone().coerce_to_dynamic()),
        )?;
    }

    #[cfg(feature = "gdal")]
    {
        let driver = crate::domain::raster::RASTER_DRIVER;
        registry.register(
            namespace,
            "raster",
            DomainGeneratorDriverResource(driver.clone().coerce_to_dynamic()),
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

fn register_pipelinestep_drivers(
    namespace: &Namespace,
    registry: &mut Registry<PipelineStepTypeDriverResource>,
) -> Result<(), RegistryError> {
    use crate::pipeline::*;

    registry.register(
        namespace,
        "filter",
        PipelineStepTypeDriverResource(filter::FILTER_DRIVER.clone()),
    )?;
    registry.register(
        namespace,
        "map",
        PipelineStepTypeDriverResource(map::MAP_DRIVER.clone()),
    )?;
    registry.register(
        namespace,
        "unset",
        PipelineStepTypeDriverResource(unset::UNSET_DRIVER.clone()),
    )?;

    Ok(())
}
