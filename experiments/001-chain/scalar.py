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

metrics = {
    "epr_register_final_len": "EPR register final length",
    "execution_time": "Simulation time (s)",
    "logical_topology_found": "Logical topology found",
    "num_events": "Total number of events",
    "bsm_prob": "BSM probability",
    "event_queue_len": "Event queue length",
    "bsm_tot": "ES operations",
    "epr_frees": "EPR free operations",
    "local_epr_misses": "Local EPR misses",
}

primary = "num_repeaters"
primary_label = "Chain size"
secondaries = {"memory_qubits": "Q", "num_pairs": "P"}

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
    ax.set_ylabel(ylabel)
    ax.set_xlabel(primary_label)
    legend = ax.get_legend()
    if legend:
        legend.set_title(title="")
    # ax.set_ylim(bottom=0.01, top=10)
    # ax.set_yscale("log")
    # plt.xticks(rotation=45)
    fig.suptitle(f"")
    plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-scalar-{metric}.{IMAGE_TYPE}")
