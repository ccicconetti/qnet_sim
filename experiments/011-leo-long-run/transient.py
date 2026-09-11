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
MAX_XTICKS = 8

basename = os.path.basename(os.getcwd())
Path(RELATIVE_OUT_DIR).mkdir(parents=True, exist_ok=True)

pd.set_option("display.show_dimensions", False)
pd.set_option("display.max_columns", None)
pd.set_option("display.max_colwidth", None)

metrics = {
    "latency": {
        "file": "app-net-latency.csv",
        "ylabel": "Latency (ms)",
        "yscale": "log",
        "ymultiplier": 1000.0,
    },
    "fidelity": {
        "file": "fidelity.csv",
        "ylabel": "Fidelity",
        "yscale": "linear",
        "ymultiplier": 1.0,
    },
}

for metric, info in metrics.items():
    print("** metric {}".format(metric))
    df_latency = pd.read_csv(
        f"{DATA_DIR}/{info['file']}",
        usecols=["node_id", "port", "time", "value"],
    )

    df_latency["value"] *= info["ymultiplier"]
    xlim = df_latency["time"].min(), df_latency["time"].max()
    ylim = df_latency["value"].min(), df_latency["value"].max()

    for port in df_latency["port"].unique():
        fig, ax = plt.subplots()
        df_local = df_latency[df_latency["port"] == port]
        sns.lineplot(
            df_local,
            x="time",
            y="value",
            errorbar=None,
            ax=ax,
        )
        print(
            "port {}: min {} max {}".format(
                port, df_local["time"].min(), df_local["time"].max()
            )
        )
        ax.grid(visible=True)
        ax.set_ylabel(info["ylabel"])
        ax.set_xlabel("Time (s)")
        ax.set_xlim(xlim)
        ax.set_ylim(ylim)
        ax.set_yscale(info["yscale"])
        legend = ax.get_legend()
        if legend:
            legend.set_title(title="")
        fig.suptitle(f"Port {port}")
        plt.savefig(
            f"{RELATIVE_OUT_DIR}/{basename}-transient-{port}-{metric}.{IMAGE_TYPE}"
        )
        plt.close()
