use pgaf::{
    config::{self, ConfigError},
    shared::STD_NAMESPACE,
};

use pgaf_engine::processor::ProcessorBuilder;
use pgaf_sdk::registry;

fn main() {
    let mut registries = registry::Registries::default();
    let namespace = registries
        .claim_namespace(STD_NAMESPACE)
        .unwrap_or_else(|_| panic!("Failed to claim '{STD_NAMESPACE}' namespace"));

    pgaf_std::init(&namespace, &mut registries).expect("Failed to initialize stdlib.");
    println!("Initialized own resources on namespace \"{}\"", namespace);

    let (workload, args) = match config::init() {
        Ok(cfg) => cfg,
        Err(ConfigError::WorkloadFileNotFound(file)) => {
            eprintln!(
                "The workload file at {} was not found.",
                file.to_str().unwrap()
            );
            return;
        }
        Err(ConfigError::WorkloadValidation(e)) => {
            eprintln!("Invalid workload: {}", e);
            return;
        }
        Err(ConfigError::WorkloadJsonParse(e)) => {
            eprintln!("Invalid workload: {}", e);
            return;
        }
        Err(ConfigError::WorkloadYamlParse(e)) => {
            eprintln!("Invalid workload: {}", e);
            return;
        }
        Err(ConfigError::IO(e)) => {
            eprintln!("{}", e);
            return;
        }
    };

    println!(
        "Loaded configuration file from {}",
        args.workload_file.canonicalize().ok().unwrap().display()
    );

    let mut processor = ProcessorBuilder::new(&registries, STD_NAMESPACE)
        .set_domain_generator(&workload.domain.r#type, &workload.domain.args)
        .expect("Failed to configure the domain generator.")
        .set_sample_size(workload.domain.sample_size)
        .set_workers(args.threads);

    for step in workload.pipeline.iter() {
        processor = processor
            .add_pipeline_step(&step.name, &step.r#type, &step.args)
            .expect("Failed to configure pipeline step.");
    }

    for sink in workload.sink.iter() {
        processor = processor
            .add_sink(&sink.name, &sink.r#type, &sink.args)
            .expect("Failed to configure sink.");
    }

    processor.build().start();
}
