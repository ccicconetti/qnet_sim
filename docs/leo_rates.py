#!/usr/bin/env python3

# generate "leo_rates.csv" with:
#
# cargo test print_leo_rates -- --ignored
#

import matplotlib.pyplot as plt
import seaborn as sns
import pandas as pd

df = pd.read_csv(f"leo_rates.csv")

df["link_type"] = df["link_type"].replace(
    {
        "OneOrbitOrbit": "O",
        "OneOrbitGround": "G",
        "TwoOrbitOrbit": "OO",
        "TwoOrbitGround": "OG",
        "TwoGroundGround": "GG",
    }
)

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
ax.set_ylim(bottom=0.1)
ax.set_yscale("log")
ax.get_legend().set_title("Link type")
plt.savefig(f"leo_rates.pdf")
