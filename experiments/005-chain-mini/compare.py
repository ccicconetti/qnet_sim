#!/usr/bin/env python3

import os
import matplotlib.pyplot as plt
import seaborn as sns
import pandas as pd
from pathlib import Path

DATA_DIR = os.environ.get("DATA_DIR", "data")
OTHER_DATA_DIR = os.environ.get("OTHER_DATA_DIR", "../004-chain-perfect/data")
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

mean_thr = (
    df.groupby(["num_qubits", "prob_local_complete"], as_index=False)["throughput"]
    .mean()
    .rename(columns={"throughput": "mean_throughput"})
)
best = mean_thr.loc[mean_thr.groupby("num_qubits")["mean_throughput"].idxmax()]
best_df = best.reset_index(drop=True)
df = df.merge(
    best[["num_qubits", "prob_local_complete"]],
    on=["num_qubits", "prob_local_complete"],
    how="inner",
).reset_index(drop=True)

df["proto"] = "sync"

df_other = pd.read_csv(f"{OTHER_DATA_DIR}/scalar.csv")
df_other["proto"] = "async"
df_other["num_qubits"] = df_other["memory_qubits"] / 2
df_other = df_other[df_other["num_pairs"] == df_other["num_pairs"].max()].copy()
df_other["throughput"] = df_other["ebit_tot"] / df_other["duration"]
df_other["latency"] *= 1000.0

metrics = {
    "ebit_prob": "ES success rate",
    "throughput": "Throughput (ebit/s)",
    "fidelity": "Fidelity",
    "latency": "Ebit latency (ms)",
    "time_slot_duration": "Time slot duration (ms)",
    "prob_local_complete": "Local completion probability",
}

ylog_metrics = {}

primary = "num_qubits"
primary_label = "Num qubits"
secondaries = {"create_path": "C:"}
other_metrics = {"ebit_prob", "throughput", "fidelity", "latency"}

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
    if metric in other_metrics:
        sns.lineplot(
            df_other,
            x=primary,
            y=metric,
            ax=ax,
            errorbar=("ci", 95),
            marker="o",
            label="HOPPER",
        )
    plt.tight_layout()
    ax.grid(visible=True)
    ax.set_ylabel(ylabel)
    ax.set_xlabel(primary_label)
    ax.set_xticks(sorted(df["num_qubits"].unique()))
    legend = ax.get_legend()
    if legend:
        legend.set_title(title="")
    if metric in ylog_metrics:
        ax.set_yscale("log")
    for _, r in df[
        (df["seed"] == df["seed"].unique()[0]) & (df["create_path"] == "C:True")
    ].iterrows():
        x = r["num_qubits"]
        y = df[metric][
            (df["num_qubits"] == r["num_qubits"])
            & (df["create_path"] == r["create_path"])
        ].mean()
        ax.annotate(
            f"p={r['prob_local_complete']:g}",
            (x, y),
            xytext=(0, -14),
            textcoords="offset points",
            ha="left",
            va="top",
            arrowprops=dict(arrowstyle="-", lw=0.8),
        )
    fig.suptitle(f"")
    plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-compare-{metric}.{IMAGE_TYPE}")
