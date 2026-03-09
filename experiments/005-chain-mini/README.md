# 005-chain-mini

## Scenario

Mini simulation, with sync protocol, of a linear chain with 3 repeaters.

## Repeatability

Requirements:

- `qnet_ll_sim` executable built in `release`

## Dataset

To execute all the experiments in this batch:

```shell
./run.sh
```

The datasets obtained at CNR can be downloaded with:

```shell
../../scripts/download-artifacts.sh
```

After execution or download, you will find the artifacts in the `data`
directory. You can produce PDF plots by running the Python scripts in this
directory.

If needed, the required Python packages can be installed with

```shell
pip install pip install seaborn matplot numpy pandas
```

(or `pip install -r requirements` to reproduce with the very same versions).