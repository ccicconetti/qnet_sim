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

df["throughput"] = df["ebit_tot"] / df["duration"]

metrics = {
    "ebit_prob": "ES success rate",
    "bsm_tot": "ES operations",
    "throughput": "Throughput (ebit/s)",
    "epr_frees": "EPR free operations",
    "local_epr_misses": "Local EPR misses",
}

ylog_metrics = {"epr_register_final_len", "epr_frees"}

df["memory_qubits"] /= 2
df["num_repeaters"] -= 1
df["num_repeaters"] /= 2

primary = "num_repeaters"
primary_label = "Num repeaters"
secondaries = {}

hue = None
if secondaries:
    for secondary, label in secondaries.items():
        df[secondary] = label + df[secondary].astype("str")
    df["hue"] = df[secondaries.keys()].agg("-".join, axis=1)
    hue = "hue"

for metric, ylabel in metrics.items():
    fig, ax1 = plt.subplots()
    ax2 = ax1.twinx()
    sns.lineplot(
        df,
        x=primary,
        y=metric,
        ax=ax1,
        errorbar=("ci", 95),
        marker="o",
        dashes=False,
        color="C0",
    )
    ax1.lines[-1].set_label(ylabel)

    sns.lineplot(
        df,
        x=primary,
        y="fidelity",
        ax=ax2,
        errorbar=("ci", 95),
        marker="D",
        dashes=False,
        color="C1",
    )
    ax2.lines[-1].set_label("Fidelity")

    h1, l1 = ax1.get_legend_handles_labels()
    h2, l2 = ax2.get_legend_handles_labels()
    ax1.legend(h1 + h2, l1 + l2, loc="upper right")

    ax1.set_ylim(bottom=0)
    ax2.set_ylim(bottom=0.5, top=1)

    ax1.grid(visible=True)
    ax1.set_ylabel(ylabel)
    ax1.set_xlabel(primary_label)
    ax1.set_xticks(sorted(df[primary].unique()))
    ax2.set_ylabel("Fidelity")

    plt.tight_layout()
    if metric in ylog_metrics:
        ax1.set_yscale("log")
    fig.suptitle(f"")
    plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-scalar-{metric}.{IMAGE_TYPE}")
