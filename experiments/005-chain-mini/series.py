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
    "fidelity": {"label": "Fidelity", "aggregates": ["mean", "p95", "p05"]},
}

ylog_metrics = {}

primary = "prob_local_complete"
primary_label = "Prob. local complete"
secondaries = {"create_path": "Include path creation: ", "num_qubits": "Num qubits: "}

aggregate_labels = {
    "mean": "Average",
    "p95": "95th percentile",
    "p05": "5th percentile",
    "count": "Count",
}

for metric, metric_data in metrics.items():
    df = pd.read_csv(f"{DATA_DIR}/{metric}-stats.csv")

    ylabel = metric_data["label"]

    hue = None
    if secondaries:
        for secondary, label in secondaries.items():
            df[secondary] = label + df[secondary].astype("str")
        df["hue"] = df[secondaries.keys()].agg("-".join, axis=1)
        hue = "hue"

    for aggregate in metric_data["aggregates"]:
        for node_id in df["node_id"].unique():
            aggregate_label = aggregate_labels[aggregate]
            fig, ax = plt.subplots()
            sns.boxplot(
                df[df["node_id"] == node_id],
                x=primary,
                y=aggregate,
                hue=hue,
                ax=ax,
            )
            ax.grid(visible=True)
            ax.set_ylabel(f"{ylabel} - {aggregate_label}")
            ax.set_xlabel(primary_label)
            legend = ax.get_legend()
            if legend:
                legend.set_title(title="")
            # ax.set_ylim(bottom=0.01, top=10)
            if metric in ylog_metrics:
                ax.set_yscale("log")
            # plt.xticks(rotation=45)
            if node_id == 0:
                fig.suptitle(f"Alice")
            elif node_id == 1:
                fig.suptitle(f"Bob")
            else:
                fig.suptitle(f"Unknown")
            plt.savefig(
                f"{RELATIVE_OUT_DIR}/{basename}-series-{metric}-{aggregate}-{node_id}.{IMAGE_TYPE}"
            )
