"""

Arguments: vehicle_file, trips_file, dist_file, num_vehicles, seed, vehicle_output, trip_output
"""
# %%
import tempfile, os
import subprocess
import pandas as pd
import numpy as np
import sys
import re

# %%
_, vehicle_file, trips_file, check_feasible_bin, site_file, battery_file, vehicle_output = sys.argv

# %%
'''
vehicle_file = "../work/preprocessed/2/group_5/30/1500/battery_1.final.vehicles.csv.gz"
trips_file = "../work/preprocessed/2/group_5/30/1500/battery_1.final.trips.csv.gz"
battery_file = "../work/preprocessed/battery_1.toml"
site_file = "../work/preprocessed/30.sites.csv"
check_feasible_bin = "../work/binaries/check_feasibility"
vehicle_output = "/tmp/vehicle.csv.gz"
'''
# %%



# %%

df_trips =  pd.read_csv(trips_file)
df_trips.index = df_trips["id"]
print("vehicle",vehicle_file, file=sys.stderr )
df_vehicles = pd.read_csv(vehicle_file)
df_vehicles.index = df_vehicles["id"]


selected_vehicle_ids = list(df_vehicles.index)

num_vehicles = df_vehicles["id"].count()




# %%

regex = r"VehiclesInfeasible\(\[((?:\d*,? ?)*)\]\)"



result = str(subprocess.check_output([
    check_feasible_bin,
    "--vehicles", vehicle_file,
    "--trips", trips_file,
    "--sites", site_file,
    "--battery" , battery_file
    ]))


filered_df_vehicles = df_vehicles.copy()
filered_df_trips = df_trips.copy()



match = re.search(regex,result)
if match != None:
    infeasible_ilocs = list(map(int,match.group(1).split(", ")))
    filered_df_vehicles.drop(index=df_vehicles.iloc[infeasible_ilocs].index.tolist(), inplace=True)


# reindex vehicles
filered_df_vehicles['index'] = range(0,len(filered_df_vehicles.index))
filered_df_vehicles.to_csv(vehicle_output, columns=["index","id","trips"], index=False)



# %%
