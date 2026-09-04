#!/usr/bin/env python3

import os
import matplotlib.pyplot as plt
import seaborn as sns
import pandas as pd
from pathlib import Path

DATA_DIR = os.environ.get("DATA_DIR", "data")
RELATIVE_OUT_DIR = os.environ.get("RELATIVE_OUT_DIR", "plots")
IMAGE_TYPE = os.environ.get("IMAGE_TYPE", "pdf")
DURATION = 60

basename = os.path.basename(os.getcwd())
Path(RELATIVE_OUT_DIR).mkdir(parents=True, exist_ok=True)

pd.set_option("display.show_dimensions", False)
pd.set_option("display.max_columns", None)
pd.set_option("display.max_colwidth", None)

df_path_len = pd.read_csv(f"{DATA_DIR}/app-path-len-stats.csv").rename(
    columns={"mean": "path_len"}
)
path_len_keys = []
for key in df_path_len.keys():
    if key == "count":
        break
    path_len_keys.append(key)
columns_to_remove = set(df_path_len.keys()) - set(path_len_keys)
columns_to_remove -= set(["path_len"])
df_path_len.drop(columns=columns_to_remove, axis=1, inplace=True)
metrics = {
    "app-net-latency": "End-to-end network latency (s)",
    "fidelity": "Fidelity",
    # "ping-latency": "Ping latency (s)",
    "occupancy": "NIC occupancy",
}

ylog_metrics = {"app-net-latency"}

# external = "num_pairs"
# suptitle = "Number of apps = "
# primary = "num_qubits"
# primary_label = "Number of qubits"
external = "num_qubits"
suptitle = "Number of qubits = "
primary = "num_pairs"
primary_label = "Number of apps"
secondary = "path_len"

# aggregates = {"mean": "Average", "p95": "95th percentile", "count": "Count"}
aggregates = {"mean": "Average"}

for external_value in df_path_len[external].unique():
    for metric, ylabel in metrics.items():
        df = pd.read_csv(f"{DATA_DIR}/{metric}-stats.csv")
        df = df[df[external] == external_value]

        if set(path_len_keys).issubset(df.columns):
            df = df.merge(
                df_path_len,
                on=path_len_keys,
                how="left",
                validate="one_to_many",
            )

        hue = None
        if secondary in df.columns:
            df["hue"] = df[secondary]
            hue = "hue"

        if metric == "app-net-latency":
            df["rate"] = df["count"] / DURATION
            grouped = df.groupby(by=["seed", "snapshot", primary, "hue"]).sum()["rate"]
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
                legend.set_title(title="Number of hops")
            ax.set_yscale("log")
            # ax.set_ylim(bottom=0.01, top=10)
            # plt.xticks(rotation=45)
            fig.suptitle(f"{suptitle}{external_value}")
            plt.savefig(
                f"{RELATIVE_OUT_DIR}/{basename}-series-{external_value}-app-throughput.{IMAGE_TYPE}"
            )

            fig, ax = plt.subplots()
            sns.histplot(
                df,
                x="path_len",
                hue=primary,
                multiple="dodge",
                # shrink=0.8,
                ax=ax,
            )
            ax.grid(visible=True)
            ax.set_ylabel("Count")
            ax.set_xlabel(f"Path length (hops)")
            legend = ax.get_legend()
            if legend:
                legend.set_title(title="Number of hops")
            # ax.set_ylim(bottom=0.01, top=10)
            # plt.xticks(rotation=45)
            fig.suptitle(f"{suptitle}{external_value}")
            plt.savefig(
                f"{RELATIVE_OUT_DIR}/{basename}-series-{external_value}-path-len.{IMAGE_TYPE}"
            )

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
            fig.suptitle(f"{suptitle}{external_value}")
            plt.savefig(
                f"{RELATIVE_OUT_DIR}/{basename}-series-{external_value}-{metric}.{IMAGE_TYPE}"
            )
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
                legend.set_title(title="Number of hops")
            # ax.set_ylim(bottom=0.01, top=10)
            if metric in ylog_metrics:
                ax.set_yscale("log")
            # plt.xticks(rotation=45)
            fig.suptitle(f"{suptitle}{external_value}")
            plt.savefig(
                f"{RELATIVE_OUT_DIR}/{basename}-series-{external_value}-{metric}-{aggregate}.{IMAGE_TYPE}"
            )
