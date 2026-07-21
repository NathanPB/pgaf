use super::{Processor, serializer::PipelineStepTypeArgsDeserializer};
use crate::context::generator::ContextGenerator;
use crate::context::value::ContextValueDeserializeSeed;
use pgaf_sdk::registry::PublicIdentifier;
use pgaf_sdk::{domain, pipeline, registry, sink};
use serde::de::DeserializeSeed;
use std::sync::Arc;
use std::thread;

#[derive(thiserror::Error, Debug)]
pub enum ProcessorBuilderError {
    #[error("No pipeline step type registered for ID '{1}' (step {0})")]
    PipelineStepTypeIdNotFound(String, PublicIdentifier),
    #[error("Failed to deserialize args for pipeline step {0}: {1}")]
    PipelineStepArgsDeserialization(String, serde_json::Error),
    #[error("No domain generator driver registered for ID '{0}'")]
    DomainGeneratorDriverNotFound(PublicIdentifier),
    #[error("Failed to create domain generator: {0}")]
    DomainGeneratorCreation(Box<dyn std::error::Error>),
    #[error("No sink type registered for ID '{1}' (sink {0})")]
    SinkTypeIdNotFound(String, PublicIdentifier),
}

pub struct NoDomainGenerator;

pub struct WithDomainGenerator(Box<dyn domain::DomainGenerator>);

pub struct ProcessorBuilder<'a, D = NoDomainGenerator> {
    workers: usize,
    domain_state: D,
    sample_size: Option<usize>,
    registries: &'a registry::Registries,
    pipeline_step_type_args_deserializer: PipelineStepTypeArgsDeserializer<'a>,
    pipeline: Vec<(pipeline::Driver, Arc<pipeline::PipelineStepTypeArgs>)>,
    sinks: Vec<(sink::Driver, serde_json::Value)>,
}

impl<'a> ProcessorBuilder<'a, NoDomainGenerator> {
    pub fn new(registries: &'a registry::Registries, default_namespace: &str) -> Self {
        Self {
            workers: 0,
            domain_state: NoDomainGenerator,
            sample_size: None,
            registries,
            pipeline_step_type_args_deserializer: PipelineStepTypeArgsDeserializer(
                ContextValueDeserializeSeed {
                    default_namespace: default_namespace.to_string(),
                    registries,
                },
            ),
            pipeline: vec![],
            sinks: vec![],
        }
    }
}

impl<'a, D> ProcessorBuilder<'a, D> {
    pub fn set_workers(mut self, workers: usize) -> Self {
        self.workers = workers;
        self
    }

    pub fn set_sample_size(mut self, sample_size: Option<usize>) -> Self {
        self.sample_size = sample_size;
        self
    }

    pub fn add_pipeline_step(
        mut self,
        name: &str,
        identifier: &PublicIdentifier,
        raw_args: &serde_json::Value,
    ) -> Result<Self, ProcessorBuilderError> {
        let args = self
            .pipeline_step_type_args_deserializer
            .clone()
            .deserialize(raw_args)
            .map_err(|e| {
                ProcessorBuilderError::PipelineStepArgsDeserialization(name.to_string(), e)
            })?
            .0;

        let driver = self
            .registries
            .reg_pipelinestep_drivers()
            .get(identifier)
            .ok_or_else(|| {
                ProcessorBuilderError::PipelineStepTypeIdNotFound(
                    name.to_string(),
                    identifier.clone(),
                )
            })?
            .0
            .clone();

        self.pipeline.push((driver, Arc::new(args)));
        Ok(self)
    }

    pub fn add_sink(
        mut self,
        name: &str,
        identifier: &PublicIdentifier,
        raw_args: &serde_json::Value,
    ) -> Result<Self, ProcessorBuilderError> {
        let driver = self
            .registries
            .reg_sink_drivers()
            .get(identifier)
            .ok_or_else(|| {
                ProcessorBuilderError::SinkTypeIdNotFound(name.to_string(), identifier.clone())
            })?
            .0
            .clone();

        self.sinks.push((driver, raw_args.clone()));
        Ok(self)
    }

    pub fn set_domain_generator(
        self,
        identifier: &PublicIdentifier,
        raw_args: &serde_json::Value,
    ) -> Result<ProcessorBuilder<'a, WithDomainGenerator>, ProcessorBuilderError> {
        let driver = self
            .registries
            .reg_domaingen_drivers()
            .get(identifier)
            .ok_or_else(|| {
                ProcessorBuilderError::DomainGeneratorDriverNotFound(identifier.clone())
            })?;

        let domain_gen = driver
            .0
            .create(raw_args.clone())
            .map_err(ProcessorBuilderError::DomainGeneratorCreation)?;

        Ok(ProcessorBuilder {
            workers: self.workers,
            domain_state: WithDomainGenerator(domain_gen),
            sample_size: self.sample_size,
            registries: self.registries,
            pipeline_step_type_args_deserializer: self.pipeline_step_type_args_deserializer,
            pipeline: self.pipeline,
            sinks: self.sinks,
        })
    }
}

impl<'a> ProcessorBuilder<'a, WithDomainGenerator> {
    pub fn build(self) -> Processor {
        let ctx_gen = ContextGenerator::new(self.domain_state.0, self.sample_size);

        let workers = if self.workers == 0 {
            thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        } else {
            self.workers
        };

        Processor {
            ctx_gen,
            pipeline: self.pipeline,
            sinks: self.sinks,
            workers,
        }
    }
}
