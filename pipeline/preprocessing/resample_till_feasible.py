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
_, vehicle_file, trips_file, check_feasible_bin, presample_vehicle_file, presample_trips_file, site_file, battery_file, seed, vehicle_output, trip_output = sys.argv

# %%
'''
vehicle_file = "../work/preprocessed/2/group_5/30/1500/battery_1.vehicles.csv.gz"
presample_vehicle_file = "../work/preprocessed/group_5/30/battery_1.feasible.vehicles.csv.gz"
trips_file = "../work/preprocessed/2/group_5/30/1500/battery_1.final.trips.csv.gz"
presample_trips_file = "../work/preprocessed/group_5/30/final.trips.csv.gz"
seed = 2
battery_file = "../work/preprocessed/battery_1.toml"
site_file = "../work/preprocessed/30.sites.csv"
check_feasible_bin = "../work/binaries/check_feasibility"
vehicle_output = "/tmp/vehicle.csv.gz"
trip_output = "/tmp/vehicle.csv.gz"
'''
# %%


randomState = np.random.RandomState(seed=int(seed))


# %%

df_trips =  pd.read_csv(trips_file)
df_trips.index = df_trips["id"]
print("vehicle",vehicle_file, file=sys.stderr )
df_vehicles = pd.read_csv(vehicle_file)
df_vehicles.index = df_vehicles["id"]


df_vehicles_full =  pd.read_csv(presample_vehicle_file)
df_vehicles_full.index = df_vehicles_full["id"]

df_trips_full =  pd.read_csv(presample_trips_file)
df_trips_full.index = df_trips_full["id"]



selected_vehicle_ids = list(df_vehicles.index)

num_vehicles = df_vehicles["id"].count()




# %%
def find_infeasible(dfvehicles, dftrips):
    tmpDir = tempfile.mkdtemp()

    local_vehicle_path = os.path.join(tmpDir, 'vehicles.csv.gz')
    local_trip_path = os.path.join(tmpDir, 'trips.csv.gz')

    dfvehicles.to_csv(local_vehicle_path)
    dftrips.to_csv(local_trip_path)

    regex = r"VehiclesInfeasible\(\[((?:\d*,? ?)*)\]\)"


    
    result = str(subprocess.check_output([
        check_feasible_bin,
        "--vehicles", local_vehicle_path,
        "--trips", local_trip_path,
        "--sites", site_file,
        "--battery" , battery_file
        ]))
    

    match = re.search(regex,result)
    if match == None:
        return []
    else:
        return list(map(int,match.group(1).split(", ")))


# %%

filered_df_vehicles = df_vehicles.copy()
filered_df_trips = df_trips.copy()

i = 0
while i < 20:
    infeasible_ilocs = find_infeasible(filered_df_vehicles,filered_df_trips)
    if infeasible_ilocs == []:
        break
    i += 1

    filered_df_vehicles.drop(index=df_vehicles.iloc[infeasible_ilocs].index.tolist(), inplace=True)
    replacements = len(filered_df_vehicles.index)
    
    fresh = df_vehicles_full[~df_vehicles_full.index.isin(filered_df_vehicles.index)].sample(n=num_vehicles - replacements, random_state=randomState)
    filered_df_vehicles = filered_df_vehicles.append(fresh)


    selected_trip_ids = []
    for vt in filered_df_vehicles["trips"]:
        for t in (vt.strip('[ ]').split(',')):
            selected_trip_ids.append(t)
    filered_df_trips = df_trips_full.loc[selected_trip_ids]

assert len(filered_df_vehicles.index) == num_vehicles

# %%

# reindex vehicles
filered_df_vehicles['index'] = range(0,len(filered_df_vehicles.index))
filered_df_vehicles.to_csv(vehicle_output, columns=["index","id","trips"], index=False)
filered_df_trips.to_csv(trip_output)


# %%
