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

df_path_len = pd.read_csv(
    f"{DATA_DIR}/app-path-len-stats.csv",
    usecols=["seed", "snapshot", "mean"],
).rename(columns={"mean": "path_len"})
num_snapshots = df_path_len["snapshot"].max() + 1
df_zeros = pd.DataFrame(
    {
        "snapshot": list(range(num_snapshots)),
        "seed": [0] * num_snapshots,
        "path_len": [0] * num_snapshots,
    }
)
df_path_len = pd.concat([df_path_len, df_zeros], ignore_index=True)
df_path_len["snapshot"] *= 10.0

fig, ax = plt.subplots()
sns.pointplot(
    df_path_len,
    x="snapshot",
    y="path_len",
    errorbar=None,
    ax=ax,
)
ax.grid(visible=True)
ax.set_xticks(
    range(
        0,
        len(df_path_len["snapshot"].unique()) + 1,
        int(len(df_path_len["snapshot"].unique()) / 5),
    )
)
ax.set_ylabel("Average path length (hops)")
ax.set_xlabel("Time (s)")
legend = ax.get_legend()
if legend:
    legend.set_title(title="")
fig.suptitle(f"")
plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-snapshots-path-len.{IMAGE_TYPE}")

path_len_keys = []
for key in df_path_len.keys():
    if key == "count":
        break
    path_len_keys.append(key)
columns_to_remove = set(df_path_len.keys()) - set(path_len_keys)
columns_to_remove -= set(["path_len"])
df_path_len.drop(columns=columns_to_remove, axis=1, inplace=True)

df = pd.read_csv(
    f"{DATA_DIR}/app-net-latency-stats.csv",
    usecols=["num_qubits", "num_pairs", "seed", "snapshot", "count"],
)
for num_qubits in df["num_qubits"].unique():
    for num_pairs in df["num_pairs"].unique():
        df_zeros = pd.DataFrame(
            {
                "num_qubits": [num_qubits] * num_snapshots,
                "num_pairs": [num_pairs] * num_snapshots,
                "snapshot": list(range(num_snapshots)),
                "seed": [0] * num_snapshots,
                "count": [0] * num_snapshots,
            }
        )
        df = pd.concat([df, df_zeros], ignore_index=True)
df["hue"] = df["num_qubits"].astype("str") + "-" + df["num_pairs"].astype("str")
df["tpt"] = df["count"] / DURATION
df = df[(df["num_pairs"] <= 20) & (df["num_qubits"] <= 50)]
df_new = df.groupby(by=["seed", "hue", "snapshot"]).sum()["tpt"].reset_index()
df_new["snapshot"] = df_new["snapshot"] * 10.0
fig, ax = plt.subplots()
sns.lineplot(
    df_new,
    x="snapshot",
    y="tpt",
    hue="hue",
    ax=ax,
    errorbar=None,
    markers=True,
    dashes=False,
)
ax.grid(visible=True)
ax.set_ylabel("App throughput (messages/s)")
ax.set_xlabel("Time (s)")
legend = ax.get_legend()
if legend:
    legend.set_title(title="#qubits-#apps")
fig.suptitle(f"")
plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-snapshots-app-throughput.{IMAGE_TYPE}")
