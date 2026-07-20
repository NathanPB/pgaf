use pgaf::{config, shared::STD_NAMESPACE};

use pgaf_engine::processor::ProcessorBuilder;
use pgaf_sdk::registry;

fn main() {
    let mut registries = registry::Registries::default();
    let namespace = registries
        .claim_namespace(STD_NAMESPACE)
        .unwrap_or_else(|_| panic!("Failed to claim '{STD_NAMESPACE}' namespace"));

    pgaf_std::init(&namespace, &mut registries).expect("Failed to initialize stdlib.");
    println!("Initialized own resources on namespace \"{}\"", namespace);

    let cfg_result = config::init();
    if let Err(e) = cfg_result {
        println!("{}", e);
        return;
    }

    let (config, args, config_file) = cfg_result.unwrap();
    println!(
        "Loaded configuration file from {}",
        config_file.canonicalize().ok().unwrap().display()
    );

    let mut processor = ProcessorBuilder::new(&registries, STD_NAMESPACE)
        .set_domain_generator(&config.domain.r#type, &config.domain.args)
        .expect("Failed to configure the domain generator.")
        .set_sample_size(config.domain.sample_size)
        .set_workers(args.workers);

    for step in config.pipeline.iter() {
        processor = processor
            .add_pipeline_step(&step.name, &step.r#type, &step.args)
            .expect("Failed to configure pipeline step.");
    }

    processor.build().start();
}
