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
        "fidelity": "Fidelity",
        "ping-latency": "Ping latency (s)",
        "occupancy": "NIC occupancy",
}

ylog_metrics = {"app-net-latency", "app-tries", "ping-latency"}

primary = "num_qubits"
primary_label = "Number of qubits"
secondaries = {}

aggregates = {"mean": "Average", "p95": "95th percentile", "count": "Count"}

for metric, ylabel in metrics.items():
    df = pd.read_csv(f"{DATA_DIR}/{metric}-stats.csv")
    df = df[df["snapshot"] <= 307]

    hue = None
    if secondaries:
        for secondary, label in secondaries.items():
            df[secondary] = label + df[secondary].astype("str")
        df["hue"] = df[secondaries.keys()].agg("-".join, axis=1)
        hue = "hue"

    if metric == "app-net-latency":
        df["rate"] = df["count"] / df["duration"]
        grouped = df.groupby(by=["seed", "snapshot", primary]).sum()["rate"]
        df_new = grouped.reset_index()
        df_new["tpt"] = df_new["rate"]
        fig, ax = plt.subplots()
        sns.boxplot(
            df_new,
            x=primary,
            y="tpt",
            hue=hue,
            ax=ax,
        )
        ax.grid(visible=True)
        ax.set_ylabel(f"App throughput (messages/s)")
        ax.set_xlabel(primary_label)
        legend = ax.get_legend()
        if legend:
            legend.set_title(title="")
        # ax.set_ylim(bottom=0.01, top=10)
        # plt.xticks(rotation=45)
        fig.suptitle(f"")
        plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-series-app-througput.{IMAGE_TYPE}")

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
        legend = ax.get_legend()
        if legend:
            legend.set_title(title=primary_label)
        fig.suptitle(f"")
        plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-series-{metric}.{IMAGE_TYPE}")
        continue

    for aggregate, aggregate_type in aggregates.items():
        fig, ax = plt.subplots()
        sns.boxplot(
            df,
            x=primary,
            y=aggregate,
            hue=hue,
            ax=ax,
        )
        ax.grid(visible=True)
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
