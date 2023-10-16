import numpy as np
import pandas as pd
from dateutil import parser
import sys
import json

vehicles_file= sys.argv[1]
trips_file = sys.argv[2]

df_vehicles = pd.read_csv(vehicles_file)

df_trips = pd.read_csv(trips_file)
df_trips.index = df_trips["id"]
# remove trips that do not belong to any vehicle
active_trips = df_vehicles["trips"].apply(lambda r: r.strip('[ ]').split(',')).sum()
df_trips = df_trips[df_trips["id"].apply(lambda r: r in active_trips)]


df_trips = df_trips.drop_duplicates("id")
def find_trip(i):
    if not i.strip() in df_trips.index:
        return None
    return df_trips.loc[i.strip()]

df_vehicles = df_vehicles.assign(
    ftrips = df_vehicles["trips"].apply(lambda r:  list(filter(lambda l: l is not None,map(find_trip,r.strip('[ ]').split(',')))))
)

df_vehicles = df_vehicles.assign(
    duration =  df_vehicles["ftrips"].apply(lambda x: (parser.parse(x[-1]["startTime"])-parser.parse(x[0]["startTime"])) )
)
df_vehicles = df_vehicles.assign(
    distance =  df_vehicles["ftrips"].apply(lambda x: sum([a.osmDistance for a in x ]) )
)

df_vehicles = df_vehicles.assign(
    count_trips =  df_vehicles["ftrips"].apply(lambda x: len(x)) 
)
df_vehicles = df_vehicles.assign(
    count_trips_customer =  df_vehicles["ftrips"].apply(lambda x: len(list(filter(lambda f: f[2] == True,x))))
)


obj = json.loads(df_vehicles[["duration","distance","count_trips","count_trips_customer"]].describe().to_json())
  
# Pretty Print JSON
json_formatted_str = json.dumps(obj, indent=4)
print(json_formatted_str)