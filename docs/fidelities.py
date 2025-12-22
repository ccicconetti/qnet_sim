#!/usr/bin/env python3

# generate "fidelities.csv" with:
#
# cargo test print_leo_fidelities -- --ignored
#

import matplotlib.pyplot as plt
import seaborn as sns
import pandas as pd

df = pd.read_csv(f"fidelities.csv")

fig, ax = plt.subplots()
sns.lineplot(
    df,
    x="distance_km",
    y="fidelity",
    hue="elevation_degrees",
    ax=ax,
    style="elevation_degrees",
    markers=True,
    dashes=False,
)
ax.grid(visible=True)
ax.set_ylabel("Generation fidelity")
ax.set_xlabel("Distance (km)")
fig.suptitle(f"")
plt.savefig(f"fidelities.pdf")
