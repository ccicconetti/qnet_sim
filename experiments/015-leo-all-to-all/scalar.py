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
df = pd.read_csv(f"{DATA_DIR}/scalar.csv")

print("total execution time (s): {}".format(df["execution_time"].sum()))

# metrics = {
#     "bsm_prob": "BSM probability",
#     "bsm_tot": "ES operations",
#     "ebit_prob": "Ebit probability",
#     "ebit_tot": "Ebit operations",
#     "epr_frees": "EPR free operations",
#     "epr_register_final_len": "EPR register final length",
#     "event_queue_len": "Event queue length",
#     "execution_time": "Simulation time (s)",
#     "fidelity": "Fidelity",
#     "latency": "Latency (s)",
#     "local_epr_misses": "Local EPR misses",
#     "logical_topology_num_edges": "Logical topology size (edges)",
#     "logical_topology_rel_size": "Logical topology relative size",
#     "num_events": "Total number of events",
# }

fig, ax = plt.subplots()
sns.ecdfplot(df, x="fidelity", ax=ax)
ax.grid(visible=True)
ax.set_ylabel("ECDF")
ax.set_xlabel("Fidelity ")
legend = ax.get_legend()
if legend:
    legend.set_title(title="")
fig.suptitle(f"")
plt.savefig(f"{RELATIVE_OUT_DIR}/{basename}-scalar-fidelity.{IMAGE_TYPE}")
