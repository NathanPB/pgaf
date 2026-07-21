use pgaf_sdk::registry::{
    DomainGeneratorDriverResource, FunctionDriverResource, Namespace,
    PipelineStepTypeDriverResource, Registries, Registry, RegistryError,
};

mod domain;
mod function;
mod pipeline;

#[tracing::instrument(level = "debug", skip_all, fields(namespace = %namespace))]
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
    use crate::domain::*;

    #[cfg(feature = "gdal")]
    registry.register(
        namespace,
        "vector",
        DomainGeneratorDriverResource(vector::VECTOR_DRIVER.clone()),
    )?;

    #[cfg(feature = "gdal")]
    registry.register(
        namespace,
        "raster",
        DomainGeneratorDriverResource(raster::RASTER_DRIVER.clone()),
    )?;

    registry.register(
        namespace,
        "rect",
        DomainGeneratorDriverResource(rect::RECT_DRIVER.clone()),
    )?;

    Ok(())
}

fn register_function_drivers(
    namespace: &Namespace,
    registry: &mut Registry<FunctionDriverResource>,
) -> Result<(), RegistryError> {
    use crate::function::*;

    registry.register(
        namespace,
        "dbgfib",
        FunctionDriverResource(dbgfib::DBG_FIB_DRIVER.clone()),
    )?;

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
        "display",
        PipelineStepTypeDriverResource(display::DISPLAY_DRIVER.clone()),
    )?;

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

    registry.register(
        namespace,
        "void",
        PipelineStepTypeDriverResource(void::VOID_DRIVER.clone()),
    )?;

    registry.register(
        namespace,
        "cmd",
        PipelineStepTypeDriverResource(cmd::CMD_DRIVER.clone()),
    )?;

    registry.register(
        namespace,
        "template",
        PipelineStepTypeDriverResource(template::TEMPLATE_DRIVER.clone()),
    )?;

    Ok(())
}
