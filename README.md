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
