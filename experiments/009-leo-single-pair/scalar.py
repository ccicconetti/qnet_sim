#!/usr/bin/env python3

import os
import matplotlib.pyplot as plt
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
df = pd.read_csv(f"{DATA_DIR}/scalar.csv")

df["logical_topology_rel_size"] = (
    df["logical_topology_num_edges"] / df["logical_topology_possible_edges"]
)

metrics = {
    "bsm_prob": "BSM probability",
    "bsm_tot": "ES operations",
    "ebit_prob": "Ebit probability",
    "ebit_tot": "Ebit operations",
    "epr_frees": "EPR free operations",
    "epr_register_final_len": "EPR register final length",
    "event_queue_len": "Event queue length",
    "execution_time": "Simulation time (s)",
    "fidelity": "Fidelity",
    "latency": "Latency (s)",
    "local_epr_misses": "Local EPR misses",
    "logical_topology_num_edges": "Logical topology size (edges)",
    "logical_topology_rel_size": "Logical topology relative size",
    "num_events": "Total number of events",
}

ylog_metrics = {"epr_register_final_len", "epr_frees", "bsm_tot", "ebit_tot", "latency"}

primary = "num_qubits"
primary_label = "Number of memory qubits"
secondaries = {"num_pairs": "#apps="}

hue = None
if secondaries:
    for secondary, label in secondaries.items():
        df[secondary] = label + df[secondary].astype("str")
    df["hue"] = df[secondaries.keys()].agg("-".join, axis=1)
    hue = "hue"

for metric, ylabel in metrics.items():
    fig, ax = plt.subplots()
    sns.boxplot(
        df,
        x=primary,
        y=metric,
        hue=hue,
        ax=ax,
    )
    ax.grid(visible=True)
    ax.set_ylabel(ylabel)
    ax.set_xlabel(primary_label)
    legend = ax.get_legend()
    if legend:
        legend.set_title(title="")
    # ax.set_ylim(bottom=0.01, top=10)
    if metric in ylog_metrics:
        ax.set_yscale("log")
    # plt.xticks(rotation=45)
    fig.suptitle(f"")
    plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-scalar-{metric}.{IMAGE_TYPE}")

fig, ax = plt.subplots()
sns.lineplot(df, x=primary, y="logical_topology_found", hue=hue, ax=ax, errorbar=None)
ax.grid(visible=True)
ax.set_ylabel("Topology found ratio")
ax.set_xlabel(primary_label)
legend = ax.get_legend()
if legend:
    legend.set_title(title="")
fig.suptitle(f"")
plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-scalar-topology_found_ratio.{IMAGE_TYPE}")
