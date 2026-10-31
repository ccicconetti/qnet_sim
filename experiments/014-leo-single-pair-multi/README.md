# 010-leo-single-pair-multi

## Scenario

Same as 010 but with using the pairs of OGS nodes with the least connectivity, with a single combo of other parameters.

## Repeatability

Requirements:

- `qnet_ll_sim` executable

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
