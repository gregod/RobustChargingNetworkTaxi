"""

Arguments: vehicle_file, trips_file, dist_file, num_vehicles, seed, vehicle_output, trip_output
"""

# %%
import pandas as pd
import numpy as np
import math
import csv
import sys
import io



_, vehicle_file, trips_file, dist_file, num_vehicles,seed, vehicle_output, trip_output = sys.argv




num_vehicles = int(num_vehicles)
randomState = np.random.RandomState(seed=int(seed))


# %%

df_trips =  pd.read_csv(trips_file)
df_trips.index = df_trips["id"]
df_vehicles = pd.read_csv(vehicle_file)
df_vehicles.index = df_vehicles["id"]
# %%
df_dist = pd.read_csv(dist_file,header=None)
df_dist["scaled"] = round(df_dist / df_dist.sum() * num_vehicles)

bucket_width_in_hours = int(24/df_dist["scaled"].count())


#df_dist

# %%
# extract only the first trip (the shift start)
df_vehicles["startTime"] = df_vehicles["trips"].apply(lambda r: df_trips.loc[r.strip('[ ]').split(',')[0]]["startTime"])

df_vehicles["startTimeComp"] = pd.DatetimeIndex(df_vehicles["startTime"]).hour // bucket_width_in_hours
groups = df_vehicles.groupby(["startTimeComp"])

selected_vehicle_ids = []


for idx,(name, group) in enumerate(groups):
    n = int(df_dist["scaled"].iloc[idx])
    for id in (group.sample(n=n,random_state=randomState)["id"]):
        selected_vehicle_ids.append(id)


# evtl manchmal nen rundungsfehler!
# dann einfach noch den rest aus der gesammt
# population (die ich noch nicht selected habe) reinfiltern
if len(selected_vehicle_ids) < num_vehicles:
    for id in df_vehicles[~df_vehicles.index.isin(selected_vehicle_ids)].sample(n=num_vehicles - len(selected_vehicle_ids), random_state=randomState)["id"]:
        selected_vehicle_ids.append(id)



# filter to sampled list
df_vehicles = df_vehicles.loc[selected_vehicle_ids].head(num_vehicles)

assert len(df_vehicles.index) == num_vehicles

# %%
selected_trip_ids = []
for vt in df_vehicles["trips"]:
    for t in (vt.strip('[ ]').split(',')):
        selected_trip_ids.append(t)

df_trips = df_trips.loc[selected_trip_ids]


# %%

df_vehicles.to_csv(vehicle_output)
df_trips.to_csv(trip_output)



# %%
