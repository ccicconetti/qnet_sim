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

metrics = {
    "app-net-latency": {
        "label": "End-to-end network latency (s)",
        "aggregates": ["mean"],
        "ylim": [0, 1],
    },
    "fidelity": {"label": "Fidelity", "aggregates": ["mean"]},
    # "ping-latency": {"label": "Ping latency (s)", "aggregates": ["mean", "count"]},
    # "occupancy": {"label": "NIC occupancy", "aggregates": ["mean"]},
}

ylog_metrics = {}

primary = "memory_qubits"
primary_label = "Num qubits"
secondaries = {"num_pairs": "Num applications = "}

aggregate_labels = {"mean": "Average", "p95": "95th percentile", "count": "Count"}

for metric, metric_data in metrics.items():
    df = pd.read_csv(f"{DATA_DIR}/{metric}-stats.csv")
    df["memory_qubits"] /= 2

    ylabel = metric_data["label"]
    ylim = metric_data["ylim"] if "ylim" in metric_data else None

    hue = None
    if secondaries:
        for secondary, label in secondaries.items():
            df[secondary] = label + df[secondary].astype("str")
        df["hue"] = df[secondaries.keys()].agg("-".join, axis=1)
        hue = "hue"

    if metric == "occupancy":
        fig, ax = plt.subplots()
        sns.ecdfplot(
            df,
            x="mean",
            hue=primary,
            ax=ax,
        )
        ax.grid(visible=True)
        ax.set_ylabel("CDF")
        ax.set_xlabel(ylabel)
        ax.set_yscale("log")
        ax.set_ylim(bottom=0.001, top=1)
        legend = ax.get_legend()
        if legend:
            legend.set_title(title=primary_label)
        fig.suptitle(f"")
        plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-series-{metric}.{IMAGE_TYPE}")
        continue

    for aggregate in metric_data["aggregates"]:
        aggregate_label = aggregate_labels[aggregate]
        fig, ax = plt.subplots()
        sns.boxplot(df, x=primary, y=aggregate, hue=hue, ax=ax)
        ax.grid(visible=True)
        ax.set_ylabel(f"{ylabel} - {aggregate_label}")
        ax.set_xlabel(primary_label)
        legend = ax.get_legend()
        if legend:
            legend.set_title(title="")
        if ylim is not None:
            ax.set_ylim(bottom=ylim[0], top=ylim[1])
        if metric in ylog_metrics:
            ax.set_yscale("log")
        # plt.xticks(rotation=45)
        fig.suptitle(f"")
        plt.savefig(
            f"{RELATIVE_OUT_DIR}/{basename}-series-{metric}-{aggregate}.{IMAGE_TYPE}"
        )
