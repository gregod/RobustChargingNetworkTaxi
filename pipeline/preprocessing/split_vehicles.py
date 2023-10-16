#!/usr/bin/env python
# coding: utf-8

"""
Takes a vehicle and trip file and splits them into daily chunks
creates an invidual folder per day in the output folder.

Arguments:  vehicle_input_file, trip_input_file, output_folder
"""

import pandas as pd
import sys
import io



output = io.StringIO()

_, vehicle_input_file, trip_input_file, output_folder = sys.argv
freq = "24h"

df_vehicles = pd.read_csv(vehicle_input_file)
df_trips = pd.read_csv(trip_input_file)
df_trips.index = df_trips["id"]
df_vehicles.index = df_vehicles["id"]




def find_trip(i):
  if not i.strip() in df_trips.index:
    return None
  return df_trips.loc[i.strip()]

vehicle_with_trips = df_vehicles["trips"].apply(lambda r:  list(filter(lambda l: l is not None,map(find_trip,r.strip('[ ]').split(',')))))
df_vehicles["trips"] = vehicle_with_trips.apply(lambda r : "[ " + ",".join(list(map(lambda t: t["id"], r))) + " ]")



df_vehicles["shiftStart"] = vehicle_with_trips.apply(lambda r: r[0]["startTime"])



time_df_vehicles = df_vehicles
time_df_vehicles.index = pd.to_datetime(time_df_vehicles["shiftStart"])
grouped_df = time_df_vehicles.groupby(pd.Grouper(freq=freq, label='left'))

for key, item in grouped_df:
 filename = key.strftime("%Y-%m-%d-%H:%M:%S")
                         
 df = grouped_df.get_group(key)
 df.index = range(0,len(df))
 df.index.name = "index"

 output_suffix = ".csv"
 if vehicle_input_file.endswith(".gz"):
     output_suffix = ".csv.gz"

 df.to_csv(output_folder + "/" + filename + output_suffix, columns=("id","trips"))

