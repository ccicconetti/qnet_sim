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

df = pd.read_csv(f"{DATA_DIR}/physical-distance-stats.csv")

df["mean"] /= 1000.0

fig, ax = plt.subplots()
sns.histplot(
    data=df[df["num_qubits"] == 50],
    x="mean",
    hue="node_type.2",
    ax=ax,
    stat="probability",
    common_norm=False,
)
ax.grid(visible=True)
ax.set_ylabel(f"Probability mass function")
ax.set_xlabel("Distance (km)")
ax.get_legend().set_title(title="")
fig.suptitle(f"")
plt.savefig(
    f"{RELATIVE_OUT_DIR}/{basename}-topology-physical-distance.{IMAGE_TYPE}"
)
