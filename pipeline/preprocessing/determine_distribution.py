"""

Arguments: trips_file, vehicle file
"""

# %%
import pandas as pd
import numpy as np
import math
import csv
import sys
import io

bucket_width_in_hours = 3

_, vehicle_file, trips_file = sys.argv


# %%

df_trips =  pd.read_csv(trips_file)
df_vehicles = pd.read_csv(vehicle_file)

# extract only the first trip (the shift start)
first_trips = df_vehicles["trips"].apply(lambda r: r.strip('[ ]').split(',')[0]).sum()
df_trips = df_trips[df_trips["id"].apply(lambda r: r in first_trips)]

# %%

start_times = pd.to_datetime(df_trips["startTime"])

# %%
# set column to hour component of start time integer divided by bucket_width_in_hours to make 24/bucket_width_in_hours = X buckets
df_trips["startTimeComp"] = pd.DatetimeIndex(start_times).hour // bucket_width_in_hours
buckets = df_trips.groupby(["startTimeComp"])["id"].count()

for b in buckets:
    print(b)


# %%
