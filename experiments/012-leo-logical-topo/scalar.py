#!/usr/bin/env python3

import os
import matplotlib.pyplot as plt
from matplotlib.ticker import MaxNLocator
import seaborn as sns
import pandas as pd
from pathlib import Path

DATA_DIR = os.environ.get("DATA_DIR", "data")
RELATIVE_OUT_DIR = os.environ.get("RELATIVE_OUT_DIR", "plots")
IMAGE_TYPE = os.environ.get("IMAGE_TYPE", "pdf")

basename = os.path.basename(os.getcwd())
Path(RELATIVE_OUT_DIR).mkdir(parents=True, exist_ok=True)

pd.set_option("display.show_dimensions", False)
pd.set_option("display.max_columns", None)
pd.set_option("display.max_colwidth", None)
df = pd.read_parquet(f"{DATA_DIR}/scalar.parquet")
df["snapshot"] = pd.to_numeric(df["snapshot"])
df["num_qubits"] = pd.to_numeric(df["num_qubits"])
df["detectors"] = pd.to_numeric(df["detectors"])

df["logical_topology_rel_size"] = (
    df["logical_topology_num_edges"] / df["logical_topology_possible_edges"]
)
df["hue"] = df["num_qubits"].astype("str") + "-" + df["detectors"].astype("str")

metrics = {
    "logical_topology_num_edges": "Logical topology size (edges)",
    "logical_topology_rel_size": "Logical topology relative size",
    "logical_topology_found": "Logical topology found ratio",
}

grouped = (
    df.groupby(["snapshot", "num_qubits", "detectors"])["logical_topology_found"]
    .mean()
    .to_frame("logical_topology_found")
)
summary = (
    grouped.groupby(["num_qubits", "detectors"])["logical_topology_found"]
    .mean()
    .reset_index()
)
for num_qubits, detectors, found_ratio in summary.itertuples(index=False, name=None):
    print(f"{num_qubits} & {detectors} & {found_ratio:.6f}" + r" \\")

for metric, xlabel in metrics.items():
    fig, ax = plt.subplots()
    sns.ecdfplot(
        df.groupby(["snapshot", "hue"])[metric].mean().reset_index(),
        x=metric,
        hue="hue",
        ax=ax,
    )
    ax.grid(visible=True)
    ax.set_ylabel("ECDF")
    ax.set_xlabel(xlabel)
    legend = ax.get_legend()
    if legend:
        legend.set_title(title="")
    fig.suptitle(f"")
    plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-scalar-{metric}.{IMAGE_TYPE}")


fig, ax = plt.subplots()
sns.lineplot(
    df[
        ((df["num_qubits"] == 10) & (df["detectors"] == 10))
        | ((df["num_qubits"] == 10) & (df["detectors"] == 20))
        | ((df["num_qubits"] == 20) & (df["detectors"] == 20))
    ],
    x="snapshot",
    y="logical_topology_found",
    ax=ax,
    hue="hue",
    errorbar=None,
)
# ax.xaxis.set_major_locator(MaxNLocator(nbins=10, integer=True))
ax.grid(visible=False, axis="x", which="both")
ax.grid(visible=True, axis="y", alpha=0.3)
ax.set_ylabel("Topology found ratio")
ax.set_xlabel("Snapshot")
legend = ax.get_legend()
if legend:
    legend.set_title(title="")
fig.suptitle(f"")
plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-scalar-topology_found_ratio.{IMAGE_TYPE}")
