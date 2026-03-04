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

df["throughput"] = df["ebit_tot"] / (df["duration"] - df["warmup_period"])
df["throughput"] *= df["num_qubits"]
df["time_slot_duration"] = 1000.0 * df["time_slot_duration"]
df["latency"] *= 1000.0

metrics = {
    "ebit_prob": "ES success rate",
    "bsm_prob": "ES operations",
    "fidelity": "Fidelity",
    "throughput": "Throughput (ebit/s)",
    "time_slot_duration": "Time slot duration (ms)",
}

ylog_metrics = {}

primary = "prob_local_complete"
primary_label = "Prob. local complete"
secondaries = {"create_path": "C:", "num_qubits": "Q:"}

hue = None
if secondaries:
    for secondary, label in secondaries.items():
        df[secondary] = label + df[secondary].astype("str")
    df["hue"] = df[secondaries.keys()].agg("-".join, axis=1)
    hue = "hue"

for metric, ylabel in metrics.items():
    fig, ax = plt.subplots()
    sns.lineplot(
        df,
        x=primary,
        y=metric,
        hue=hue,
        style=hue,
        ax=ax,
        errorbar=("ci", 95),
        markers=True,
        dashes=False,
    )
    ax.grid(visible=True)
    ax.set_ylabel(ylabel)
    ax.set_xlabel(primary_label)
    legend = ax.get_legend()
    if legend:
        legend.set_title(title="")
        legend.set_loc(loc="center left")
        legend.set_bbox_to_anchor((1.02, 0.5))
    plt.tight_layout()
    if metric in ylog_metrics:
        ax.set_yscale("log")
    fig.suptitle(f"")
    plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-scalar-{metric}.{IMAGE_TYPE}")
