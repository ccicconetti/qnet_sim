#!/usr/bin/env python3

# generate "leo_rates.csv" with:
#
# cargo test print_leo_rates -- --ignored
#

import matplotlib.pyplot as plt
import seaborn as sns
import pandas as pd

df = pd.read_csv(f"leo_rates.csv")

fig, ax = plt.subplots()
sns.lineplot(
    df,
    x="distance_km",
    y="rate",
    hue="link_type",
    ax=ax,
    style="link_type",
    markers=True,
    dashes=False,
)
ax.grid(visible=True)
ax.set_ylabel("Rate (EPR/s)")
ax.set_xlabel("Distance (km)")
ax.set_ylim(bottom=0)
plt.savefig(f"leo_rates.pdf")
