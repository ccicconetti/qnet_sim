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
    "app-net-latency": "End-to-end network latency (s)",
    "app-path-len": "Path length (hops)",
    "app-tries": "Number of retries",
    "fidelity": "Fidelity",
    "ping-latency": "Ping latency (s)",
}

ylog_metrics = {"app-net-latency", "app-tries", "ping-latency"}

primary = "num_repeaters"
primary_label = "Chain size"
secondaries = {"memory_qubits": "Q", "num_pairs": "P"}

aggregates = {"mean": "Average", "p95": "95th percentile"}

for metric, ylabel in metrics.items():
    df = pd.read_csv(f"{DATA_DIR}/{metric}-stats.csv")

    hue = None
    if secondaries:
        for secondary, label in secondaries.items():
            df[secondary] = label + df[secondary].astype("str")
        df["hue"] = df[secondaries.keys()].agg("-".join, axis=1)
        hue = "hue"

    for aggregate, aggregate_type in aggregates.items():
        fig, ax = plt.subplots()
        sns.boxplot(
            df,
            x=primary,
            y=aggregate,
            hue=hue,
            ax=ax,
        )
        ax.set_ylabel(f"{ylabel} - {aggregate_type}")
        ax.set_xlabel(primary_label)
        legend = ax.get_legend()
        if legend:
            legend.set_title(title="")
        # ax.set_ylim(bottom=0.01, top=10)
        if metric in ylog_metrics:
            ax.set_yscale("log")
        # plt.xticks(rotation=45)
        fig.suptitle(f"")
        plt.savefig(
            f"{RELATIVE_OUT_DIR}/{basename}-series-{metric}-{aggregate}.{IMAGE_TYPE}"
        )
