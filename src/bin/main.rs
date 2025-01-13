// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use clap::Parser;
use qnet_ll_sim::config::Config;
use qnet_ll_sim::simulation::Simulation;
use qnet_ll_sim::user_config::UserConfig;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    /// Simulation configuration.
    #[arg(long, short, default_value_t = String::from("conf.json"))]
    conf: String,
    /// Create a template for the simulation configuration.
    #[arg(long, short)]
    template: bool,
    /// Initial seed to initialize the pseudo-random number generators
    #[arg(long, default_value_t = 0)]
    seed_init: u64,
    /// Final seed to initialize the pseudo-random number generators
    #[arg(long, default_value_t = 10)]
    seed_end: u64,
    /// Number of parallel workers
    #[arg(long, default_value_t = std::thread::available_parallelism().unwrap().get())]
    concurrency: usize,
    /// Name of the path where to save the metrics collected.
    #[arg(long, default_value_t = String::from("data/"))]
    output_path: String,
    /// Append to the output file.
    #[arg(long, default_value_t = false)]
    append: bool,
    /// Additional fields recorded in the CSV output file.
    #[arg(long, default_value_t = String::from(""))]
    additional_fields: String,
    /// Header of additional fields recorded in the CSV output file.
    #[arg(long, default_value_t = String::from(""))]
    additional_header: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();

    // If requested, save a template configuration file and quit.
    let conf_path = std::path::Path::new("conf.json");
    if args.template {
        if conf_path.exists() {
            log::warn!("File {:#?} exists and will not be overwritten", conf_path);
        } else {
            std::fs::write(
                conf_path,
                serde_json::to_string_pretty(&UserConfig::default())?,
            )?;
            return Ok(());
        }
    }

    // Check command-line arguments.
    anyhow::ensure!(
        args.additional_fields.matches(',').count() == args.additional_header.matches(',').count(),
        "--additional_fields and --additional_header have a different number of commas"
    );

    // Read the user's configuration file.
    anyhow::ensure!(
        conf_path.exists(),
        "Configuration file {:#?} does not exist",
        conf_path
    );
    let conf_file = std::fs::File::open(args.conf)?;
    let reader = std::io::BufReader::new(conf_file);
    let user_config: UserConfig = serde_json::from_reader(reader)?;

    // Create the configurations of all the experiments
    let configurations = std::sync::Arc::new(std::sync::Mutex::new(vec![]));
    for seed in args.seed_init..args.seed_end {
        configurations.lock().unwrap().push(Config {
            seed,
            user_config: user_config.clone(),
        });
    }

    if configurations.lock().unwrap().is_empty() {
        return Ok(());
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    for i in 0..std::cmp::min(args.concurrency, (args.seed_end - args.seed_init) as usize) {
        let tx = tx.clone();
        let configurations = configurations.clone();
        tokio::spawn(async move {
            log::info!("spawned worker #{}", i);
            loop {
                let config;
                {
                    if let Some(val) = configurations.lock().unwrap().pop() {
                        config = Some(val);
                    } else {
                        break;
                    }
                }
                match Simulation::new(config.unwrap()) {
                    Ok(mut sim) => tx.send(sim.run()).unwrap(),
                    Err(err) => log::error!("error when running simulation: {}", err),
                };
            }
            log::info!("terminated worker #{}", i);
        });
    }
    let _ = || tx;

    // wait until all the simulations have been done
    let mut outputs = vec![];
    while let Some(output) = rx.recv().await {
        outputs.push(output);
    }

    // save output to files
    assert!(!outputs.is_empty());
    qnet_ll_sim::output::save_outputs(
        outputs,
        &args.output_path,
        args.append,
        &args.additional_header,
        &args.additional_fields,
    )
}
