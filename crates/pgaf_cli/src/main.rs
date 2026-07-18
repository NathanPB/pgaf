use pgaf::config;

use pgaf_engine::processor::ProcessorBuilder;
use pgaf_sdk::registry;

static STD_NAMESPACE: &str = "std";

fn main() {
    let mut registries = registry::Registries::default();
    let namespace = registries
        .claim_namespace(STD_NAMESPACE)
        .unwrap_or_else(|_| panic!("Failed to claim '{STD_NAMESPACE}' namespace"));

    pgaf_std::init(&namespace, &mut registries).expect("Failed to initialize stdlib.");
    println!("Initialized own resources on namespace \"{}\"", namespace);

    let cfg_seed = config::ConfigDeserializeSeed::new(&registries, &namespace);

    let cfg_result = config::init(cfg_seed);
    if let Err(e) = cfg_result {
        println!("{}", e);
        return;
    }

    let (config, args, config_file) = cfg_result.unwrap();
    println!(
        "Loaded configuration file from {}",
        config_file.canonicalize().ok().unwrap().display()
    );

    let processing = ProcessorBuilder {
        config: &config,
        workers: args.workers,
        registries: &registries,
        std_namespace: STD_NAMESPACE.to_string(),
    }
    .build()
    .unwrap();

    processing.start();
}
