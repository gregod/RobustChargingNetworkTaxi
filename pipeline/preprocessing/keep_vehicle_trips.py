import sys
import pandas as pd

"""
Keep only the trips that are used by the vehicle file!
"""

_, vehicles_file, trips_file, output_trips = sys.argv


df_vehicles = pd.read_csv(vehicles_file)
df_trips = pd.read_csv(trips_file)
df_trips.index = df_trips["id"]

selected_trip_ids = []
for vt in df_vehicles["trips"]:
    for t in (vt.strip('[ ]').split(',')):
        selected_trip_ids.append(t)

df_trips = df_trips.loc[selected_trip_ids]
df_trips.to_csv(output_trips)