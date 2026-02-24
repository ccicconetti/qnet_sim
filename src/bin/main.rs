// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use clap::Parser;
use qnet_ll_sim::config::Config;
use qnet_ll_sim::full_config::{FullConfig, UserConfigRecipe};
use qnet_ll_sim::mini_config::MiniConfig;
use qnet_ll_sim::mini_simulation::MiniSimulation;
use qnet_ll_sim::simulation::Simulation;
use qnet_ll_sim::utils::CsvFriend;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    /// Simulation configuration.
    #[arg(long, short, default_value_t = String::from("conf.json"))]
    conf: String,
    /// Create a template for the simulation configuration. Possible values: chain, grid, leo, mini.
    #[arg(long, short, default_value_t = Default::default())]
    template: String,
    /// Initial seed to initialize the pseudo-random number generators
    #[arg(long, default_value_t = 0)]
    seed_init: u64,
    /// Run a mini simulation instead of the full one.
    #[arg(long, default_value_t = false)]
    mini: bool,
    /// Final seed to initialize the pseudo-random number generators
    #[arg(long, default_value_t = 1)]
    seed_end: u64,
    /// Number of parallel workers
    #[arg(long, default_value_t = std::thread::available_parallelism().unwrap().get())]
    concurrency: usize,
    /// Save to Dot files and quit.
    #[arg(long)]
    save_to_dot: bool,
    /// Print the available metrics and quit.
    #[arg(long)]
    print_metrics: bool,
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
    /// Add the configuration values to the CSV output file.
    #[arg(long)]
    save_config: bool,
    /// Save the samples of time series.
    #[arg(long)]
    save_time_series: bool,
    /// Print the version number and quit.
    #[arg(long, default_value_t = false)]
    version: bool,
}

#[derive(Clone)]
enum SimConfig {
    Full(FullConfig),
    Mini(MiniConfig),
}

impl CsvFriend for SimConfig {
    fn header(&self) -> String {
        match self {
            SimConfig::Full(full_config) => full_config.header(),
            SimConfig::Mini(mini_config) => mini_config.header(),
        }
    }
    fn to_csv(&self) -> String {
        match self {
            SimConfig::Full(full_config) => full_config.to_csv(),
            SimConfig::Mini(mini_config) => mini_config.to_csv(),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();

    if args.version {
        println!(
            "{}.{}.{}{}{} ({})",
            env!("CARGO_PKG_VERSION_MAJOR"),
            env!("CARGO_PKG_VERSION_MINOR"),
            env!("CARGO_PKG_VERSION_PATCH"),
            if env!("CARGO_PKG_VERSION_PRE").is_empty() {
                ""
            } else {
                "-"
            },
            env!("CARGO_PKG_VERSION_PRE"),
            git_version::git_version!()
        );
        return Ok(());
    }

    // If requested, save a template configuration file and quit.
    let conf_path = std::path::Path::new(&args.conf);
    if !args.template.is_empty() {
        if conf_path.exists() {
            println!("File {:#?} exists and will not be overwritten", conf_path);
        } else {
            if args.mini || args.template == "mini" {
                std::fs::write(
                    conf_path,
                    serde_json::to_string_pretty(&MiniConfig::default())?,
                )?;
            } else {
                std::fs::write(
                    conf_path,
                    serde_json::to_string_pretty(&FullConfig::default_with_recipe(
                        UserConfigRecipe::from_str(&args.template)?,
                    ))?,
                )?;
            }
        }
        return Ok(());
    }

    // Check command-line arguments.
    anyhow::ensure!(
        args.additional_fields.matches(',').count() == args.additional_header.matches(',').count(),
        "--additional_fields and --additional_header have a different number of commas"
    );
    anyhow::ensure!(
        !args.save_to_dot || (args.seed_end - args.seed_init) == 1,
        "cannot use --save-to-dot with multiple seeds"
    );

    // Read the user's configuration file.
    anyhow::ensure!(
        conf_path.exists(),
        "Configuration file {:#?} does not exist",
        conf_path
    );
    let conf_file = std::fs::File::open(conf_path)?;
    let reader = std::io::BufReader::new(conf_file);

    let sim_config = if args.mini {
        let mini_config: MiniConfig = serde_json::from_reader(reader)?;
        SimConfig::Mini(mini_config)
    } else {
        let full_config: FullConfig = serde_json::from_reader(reader)?;
        SimConfig::Full(full_config)
    };

    // Create the configurations of all the experiments
    let configurations = std::sync::Arc::new(std::sync::Mutex::new(vec![]));
    for seed in args.seed_init..args.seed_end {
        let config = Config { seed };

        configurations.lock().unwrap().push(config);
    }

    let (config_csv_header, user_config_csv_header) = {
        let lock = configurations.lock().unwrap();
        if let Some(config) = lock.first() {
            (config.header(), sim_config.header())
        } else {
            return Ok(());
        }
    };

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    for i in 0..std::cmp::min(args.concurrency, (args.seed_end - args.seed_init) as usize) {
        let tx = tx.clone();
        let configurations = configurations.clone();
        let sim_config = sim_config.clone();
        tokio::spawn(async move {
            log::info!("spawned worker #{}", i);
            loop {
                let config = {
                    let mut lock = configurations.lock().unwrap();
                    if let Some(config) = lock.pop() {
                        config
                    } else {
                        break;
                    }
                };
                match &sim_config {
                    SimConfig::Full(full_config) => match Simulation::new(
                        config,
                        full_config.clone(),
                        args.save_to_dot,
                        args.print_metrics,
                    ) {
                        Ok(mut sim) => tx.send(sim.run()).unwrap(),
                        Err(err) => log::error!("error when running simulation: {}", err),
                    },
                    SimConfig::Mini(mini_config) => {
                        match MiniSimulation::new(config, mini_config.clone(), args.print_metrics) {
                            Ok(mut sim) => tx.send(sim.run()).unwrap(),
                            Err(err) => log::error!("error when running simulation: {}", err),
                        }
                    }
                }
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
    if !outputs.is_empty() {
        qnet_ll_sim::output::save_outputs(
            outputs,
            qnet_ll_sim::output::OutputSaveConf {
                output_path: args.output_path,
                append: args.append,
                config_csv_header,
                user_config_csv_header,
                additional_header: args.additional_header,
                additional_fields: args.additional_fields,
                save_config: args.save_config,
                save_time_series: args.save_time_series,
            },
        )?;
    }

    Ok(())
}
