"""

"""
import datetime
import pandas as pd
import numpy as np
from dateutil import parser
import math
import sys
import csv
import io

output = io.StringIO()

_, vehicle_input_file , trip_input_file, max_hours = sys.argv

df_vehicles = pd.read_csv(vehicle_input_file)
df_vehicles.index = df_vehicles["id"]
df_trips = pd.read_csv(trip_input_file)
df_trips.index = df_trips["id"]

df_trips["startTime"] = list(df_trips["startTime"].map(lambda x: parser.parse(x)))
df_trips["endTime"] = list(df_trips["endTime"].map(lambda x: parser.parse(x)))
df_trips = df_trips.drop_duplicates("id")
def find_trip(i):
    if not i.strip() in df_trips.index:
        return None
    return df_trips.loc[i.strip()]

vehicle_with_trips = df_vehicles.assign(
    ftrips = df_vehicles["trips"].apply(lambda r:  list(filter(lambda l: l is not None,map(find_trip,r.strip('[ ]').split(',')))))
)

vehicle_with_trips = vehicle_with_trips.assign(
    tripStartTime = vehicle_with_trips["ftrips"].apply(lambda r: r[0]["startTime"])
)

remove_trips = []
remove_vehicles = []

# identify vehicles where total tour length is larger than max_hours
for vi,v in vehicle_with_trips.iterrows():
    for trip in v["ftrips"]:
        # remove vehicles where individual trips are longer than 2 hours
        if trip["endPeriod"] - trip["startPeriod"] > 2 * 60 / 5:
            remove_vehicles.append(v["id"])
        if( (trip["endTime"]-v["tripStartTime"]).seconds//3600 > int(max_hours)  ):
            remove_trips.append(trip.id)




# filter vehicles
df_vehicles = df_vehicles[~df_vehicles.index.isin(remove_vehicles)]

df_vehicles["trips"] = vehicle_with_trips["ftrips"].apply(lambda r : "[ " + ",".join(list(filter(lambda t: t not in remove_trips ,map(lambda t: t["id"], r)))) + " ]")
df_vehicles.to_csv(output,index=False)
print(output.getvalue())
