#!/usr/bin/env python3

# generate "leo_fidelities.csv" with:
#
# cargo test print_leo_fidelities -- --ignored
#

import matplotlib.pyplot as plt
import seaborn as sns
import pandas as pd

df = pd.read_csv(f"leo_fidelities.csv")

for h_b in df["h_b"].unique():
    fig, ax = plt.subplots()
    sns.lineplot(
        df[df["h_b"] == h_b],
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
    ax.set_ylim(bottom=0.25, top=1)
    ax.set_title(f"$H_b = {h_b}~W \\cdot m^{{-2}} \\cdot sr^{{-1}} \\cdot m^{{-1}}$")
    plt.savefig(f"leo_fidelities-{h_b}.pdf")
