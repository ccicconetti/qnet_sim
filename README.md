# qnet_ll_sim

Quantum network simulator focused on modeling network-level protocols and
their interactions with networked quantum applications.

Some pre-configured experiments can be found in the directory `experiments`.

## Building instructions

Install dependencies, if using Ubuntu:

```bash
sudo apt update && sudo apt install curl git make gcc -y
```

Clone repository:

```bash
git clone https://github.com/ccicconetti/qnet_ll_sim.git
cd qnet_ll_sim
```

Install Rust (follow the interactive instructions):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Reload your environment, with Bash:

```bash
. "$HOME/.cargo/env"
```

Compile in release mode:

```bash
cargo build --release
```

This will create the main executable as `target/release/qnet_ll_sim`.

### Example execution

Build a simple example configuration with:

```bash
target/release/qnet_ll_sim -t chain
```

This will create a JSON file `conf.json` that contains the specifications of
a simple experiment with two on-ground stations and one satellite acting as
repeater.

Then execute the simulator:

```bash
target/release/qnet_ll_sim
```

This will produce the simulation results in CSV files in `data`.

To export Apache Parquet files instead, select the output format:

```bash
target/release/qnet_ll_sim --output-format parquet --save-time-series
```

CSV remains the default, and can also be selected with `--output-format csv`.
Parquet export uses the official native Rust `parquet` library with Snappy
compression. It writes `scalar.parquet` and `<metric>-stats.parquet`, plus
`<metric>.parquet` when `--save-time-series` is set. Metrics without samples
do not produce series or statistics files.

Measurements, timestamps, and summary statistics are double-precision columns;
sample counts are unsigned 64-bit integers. Labels, seed/configuration fields,
and additional fields are UTF-8 strings, preserving their existing text values.
The `--save-config`, `--additional-header`, `--additional-fields`, and
`--output-path` options apply to both formats.

With `--append`, Parquet export requires matching column names and types. It
copies existing row groups and writes the new rows to a temporary file before
replacing each output file, so appending takes time proportional to the existing
file size. Without `--append`, output files are overwritten as with CSV.

### Development

When modifying the code, it is recommend to compile in debug mode:

```bash
cargo build
```

Also, run the unit tests after every change to ensure non-regression (and add
new unit tests associated with each new feature):

```bash
cargo test
```

## License

The Repository is licensed under the MIT License. Please refer to
[LICENSE](LICENSE) and [CONTRIBUTORS.txt](CONTRIBUTORS.txt). 
