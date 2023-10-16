"""
Takes an vehicle file and retains only the top N vehicles
Optional Vehicle blacklist in code

Arguments:  vehicle_input_file , n_count
"""
import pandas as pd

import sys
import io

VEHICLE_BLACKLIST=[]

output = io.StringIO()

_, vehicle_input_file , n_count = sys.argv
n_count = int(n_count)
vehicle_blacklist = list(map(lambda i : "v" + i, VEHICLE_BLACKLIST))
df_vehicles = pd.read_csv(vehicle_input_file)
df_vehicles = df_vehicles[(df_vehicles["id"].apply(lambda id: id not in vehicle_blacklist))]
df_vehicles = df_vehicles.head(n_count)
df_vehicles.to_csv(output)



print(output.getvalue())