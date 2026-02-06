# 002-leo-num_qubits

## Scenario

Multiple time snapshots of a LEO constellation.

In each snapshot, an infinite ping between any two OGSs is established.

A realistic model is used to determine the rate of generation of EPRs and their
fidelity.

Primary factor:

- number of qubits per node

## Repeatability

Requirements:

- `qnet_ll_sim` executable (or symlink)

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
`pip install -r requirements`.
