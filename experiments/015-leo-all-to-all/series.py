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
    "app-tries": "Entanglement attempts",
    "occupancy": "NIC occupancy",
}

ylog_metrics = {"app-net-latency", "app-tries"}

# aggregates = {"mean": "Average", "p95": "95th percentile", "count": "Count"}
aggregates = {"mean": "Average"}

first = True
for metric, ylabel in metrics.items():
    df = pd.read_csv(f"{DATA_DIR}/{metric}-stats.csv")

    if set(path_len_keys).issubset(df.columns):
        df = df.merge(
            df_path_len,
            on=path_len_keys,
            how="left",
            validate="one_to_many",
        )

    for aggregate, aggregate_type in aggregates.items():
        fig, ax = plt.subplots(figsize=(12, 4))
        sns.stripplot(
            df,
            x="snapshot",
            y=aggregate,
            jitter=0.5,  # Controls the amount of random displacement
            size=2,  # Size of the markers
            alpha=0.5,  # Opacity to handle density
            ax=ax,
        )
        ax.grid(visible=True)
        ax.set_ylabel(f"{ylabel} - {aggregate_type}")
        ax.set_xlabel("Snapshot")
        ax.set_xticks(range(0, 501, 50))
        if first == True:
            first = False
        else:
            ax.set_xlabel("")
        # ax.set_ylim(bottom=0.01, top=10)
        if metric in ylog_metrics:
            ax.set_yscale("log")
        # plt.xticks(rotation=45)
        plt.savefig(
            f"{RELATIVE_OUT_DIR}/{basename}-series-{metric}-{aggregate}.{IMAGE_TYPE}"
        )
