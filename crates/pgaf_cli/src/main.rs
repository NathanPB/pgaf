#![feature(mpmc_channel)]

mod config;
mod processing;
mod workdir;

use crate::processing::ProcessingBuilder;
use crate::workdir::make_workdir;
use pgaf_sdk::registry;

fn main() {
    let mut registries = registry::Registries::new();
    let namespace = registries
        .claim_namespace("std")
        .expect("Failed to claim 'std' namespace.");

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

    let (workdir, temp_wd) =
        match make_workdir(&args.workdir, &args.keep_workdir, args.clear_workdir) {
            Ok(workdir) => workdir,
            Err(e) => {
                println!("Unable to validate working directory: {}", e);
                return;
            }
        };

    println!(
        "Initialized working directory at {}{}",
        workdir.display(),
        if temp_wd { " (temporary)" } else { "" }
    );

    let processing = ProcessingBuilder {
        config: &config,
        args: &args,
        workdir,
    }
    .build()
    .unwrap();

    processing.start();
}
