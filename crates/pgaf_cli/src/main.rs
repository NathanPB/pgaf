use std::process::ExitCode;

use pgaf::{
    config::{self, Args, ConfigError, Workload},
    shared::STD_NAMESPACE,
};

use pgaf_engine::processor::{Processor, ProcessorBuilder, ProcessorBuilderError};
use pgaf_sdk::registry::{self, Registries};
use tracing::instrument;

fn main() -> ExitCode {
    let args = config::parse_args();
    pgaf::trace::init(args.verbose, args.quiet);
    run(&args)
}

#[instrument(skip_all, fields(workload = %args.workload_file.display(), workers = args.threads))]
fn run(args: &Args) -> ExitCode {
    tracing::info!("initialize pgaf");

    let mut registries = registry::Registries::default();
    let namespace = match registries.claim_namespace(STD_NAMESPACE) {
        Ok(namespace) => namespace,
        Err(e) => {
            tracing::error!(error = %e, "namespace claim failed");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = pgaf_std::init(&namespace, &mut registries) {
        tracing::error!(error = %e, "stdlib init failed");
        return ExitCode::FAILURE;
    }

    tracing::debug!(namespace = %namespace, "stdlib registered");

    let workload = match config::load_workload(&args.workload_file) {
        Ok(workload) => workload,
        Err(ConfigError::WorkloadFileNotFound(file)) => {
            tracing::error!(path = %file.display(), "workload file not found");
            return ExitCode::from(2);
        }
        Err(ConfigError::WorkloadValidation(e)) => {
            tracing::error!(error = %e, "workload validation failed");
            return ExitCode::from(2);
        }
        Err(ConfigError::WorkloadJsonParse(e)) => {
            tracing::error!(error = %e, "workload parse failed");
            return ExitCode::from(2);
        }
        Err(ConfigError::WorkloadYamlParse(e)) => {
            tracing::error!(error = %e, "workload parse failed");
            return ExitCode::from(2);
        }
        Err(ConfigError::IO(e)) => {
            tracing::error!(error = %e, "workload read failed");
            return ExitCode::from(2);
        }
    };

    tracing::debug!(path = %args.workload_file.display(), "workload loaded");

    match build_processor(args, &workload, &registries) {
        Ok(processor) => processor.start(),
        Err(ProcessorBuilderError::PipelineStepTypeIdNotFound(step_name, id)) => {
            tracing::error!(step.name = %step_name, step.r#type = %id, "pipeline step type not found");
            return ExitCode::FAILURE;
        }
        Err(ProcessorBuilderError::PipelineStepArgsDeserialization(step_name, e)) => {
            tracing::error!(step.name = %step_name, error = %e, "pipeline step args deserialization failed");
            return ExitCode::FAILURE;
        }
        Err(ProcessorBuilderError::DomainGeneratorDriverNotFound(id)) => {
            tracing::error!(domain.r#type = %id, "domain driver not found");
            return ExitCode::FAILURE;
        }
        Err(ProcessorBuilderError::DomainGeneratorCreation(e)) => {
            tracing::error!(error = %e, "domain generator creation failed");
            return ExitCode::FAILURE;
        }
        Err(ProcessorBuilderError::SinkTypeIdNotFound(sink_name, id)) => {
            tracing::error!(sink.name = %sink_name, sink.r#type = %id, "sink type not found");
            return ExitCode::FAILURE;
        }
    };

    tracing::info!("workload complete");
    ExitCode::SUCCESS
}

#[instrument(skip_all)]
fn build_processor(
    args: &Args,
    workload: &Workload,
    registries: &Registries,
) -> Result<Processor, ProcessorBuilderError> {
    let mut processor = ProcessorBuilder::new(registries, STD_NAMESPACE)
        .set_domain_generator(&workload.domain.r#type, &workload.domain.args)?
        .set_sample_size(workload.domain.sample_size)
        .set_workers(args.threads);

    for step in workload.pipeline.iter() {
        processor = processor.add_pipeline_step(&step.name, &step.r#type, &step.args)?;
        tracing::debug!(step.name = %step.name, step.r#type = %step.r#type, "pipeline step configured");
    }

    for sink in workload.sink.iter() {
        processor = processor.add_sink(&sink.name, &sink.r#type, &sink.args)?;
        tracing::debug!(sink.name = %sink.name, sink.r#type = %sink.r#type, "sink configured");
    }

    Ok(processor.build())
}
