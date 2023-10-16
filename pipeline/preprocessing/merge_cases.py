"""
Takes two vehicle and trip files and merges them into one
Arguments: v1 v2 t1 t2 output_v output_t
"""
import pandas as pd
import re
import sys
import csv
import io


inputs = sys.argv[1:]

num_vehicles = int((len(inputs) - 2) / 2)
#v1 v2 t1 t2 o1 o2

data_tuple = [(inputs[i],inputs[i+num_vehicles]) for i in range(0,num_vehicles) ]


out_vehicles=inputs[-2]
out_trips=inputs[-1]



merged_df_vehicles = pd.read_csv(data_tuple[0][0])
df_merged_trips = pd.read_csv(data_tuple[0][1])

for vehicle_file,trip_file in data_tuple[1:]:

    vehicles = pd.read_csv(vehicle_file)
    trips = pd.read_csv(trip_file)
    # add infeasible vehicles to new set.
    merged_df_vehicles = merged_df_vehicles.append(vehicles)
    df_merged_trips = df_merged_trips.append(trips)



merged_df_vehicles.index = merged_df_vehicles["index"] = range(0,len(merged_df_vehicles))

merged_df_vehicles.to_csv(path_or_buf=out_vehicles, quoting=csv.QUOTE_MINIMAL, index=False)
df_merged_trips.to_csv(path_or_buf=out_trips,quoting=csv.QUOTE_NONNUMERIC, index=False)