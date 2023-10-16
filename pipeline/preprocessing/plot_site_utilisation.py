import pandas as pd
import matplotlib.pyplot as plt
import sys

_, process_file, site_file, output_file = sys.argv

processes=pd.read_csv(process_file, header=None,names=("Vehicle","Site","Time"))
processes = processes.sort_values(by=["Vehicle","Time"])

df_sites = pd.read_csv(site_file)
df_sites.index = df_sites.apply(lambda d : int(d["id"].replace("s","")),axis=1)

sites = sorted(list(set(processes["Site"])))
time = range(0,int(processes["Time"].max()))
fig, ax = plt.subplots(len(sites), 1,figsize=(18,len(sites) * 0.9),sharex="all",sharey="all", constrained_layout=True)
fig.suptitle("Active sites and their utilization")
for idx,site in enumerate(sites):
    data=processes[processes["Site"]==site].groupby("Time")["Site"].count()
    data = data.reindex(range(0, max(data.index)), fill_value=0)
    time = range(0,len(data))
    ax[idx].bar(time,data)
    ax[idx].set_yticks([0,2,4])
    ax[idx].set_ylim([0, 4])
    ax[idx].axhline(y=df_sites.loc[site]["capacity"], color='r', linestyle='-')
    ax[idx].set_ylabel(f"Site {site}")
fig.savefig(output_file)