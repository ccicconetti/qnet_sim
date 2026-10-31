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

df = pd.read_csv(
    f"{DATA_DIR}/app-net-latency-stats.csv",
    usecols=["bob", "seed", "snapshot", "count"],
)
num_snapshots = df["snapshot"].max() + 1
df["bob"] = df["bob"].map({38: 7, 37: 2, 40: 1})
for bob in df["bob"].unique():
    df_zeros = pd.DataFrame(
        {
            "bob": [bob] * num_snapshots,
            "snapshot": list(range(num_snapshots)),
            "seed": [0] * num_snapshots,
            "count": [0] * num_snapshots,
        }
    )
    df = pd.concat([df, df_zeros], ignore_index=True)
df["hue"] = df["bob"]
df["tpt"] = df["count"] / DURATION
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
ax.set_xlabel("Snapshot")
legend = ax.get_legend()
if legend:
    legend.set_title(title="OGS ID")
fig.suptitle(f"")
plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-snapshots-app-throughput.{IMAGE_TYPE}")
