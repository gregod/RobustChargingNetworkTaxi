"""
Takes a vehicle and trip file and transforms start and endtimes to periods
Preprocessing is applied to make sure that periods do not overlap

Arguments: vehicle_input_file , trip_input_file, vehicle_output_file

Outputs trip_file to stdout
"""
import datetime

import pandas as pd
import numpy as np

from dateutil import parser
import math


min_per_period = 5

import sys
import csv
import io

output = io.StringIO()

_, vehicle_input_file , trip_input_file, vehicle_output_file = sys.argv

df_vehicles = pd.read_csv(vehicle_input_file)
df_trips = pd.read_csv(trip_input_file)
df_trips.index = df_trips["id"]


# remove trips that do not belong to any vehicle
active_trips = df_vehicles["trips"].apply(lambda r: r.strip('[ ]').split(',')).sum()
df_trips = df_trips[df_trips["id"].apply(lambda r: r in active_trips)]


df_trips["startTime"] = list(df_trips["startTime"].map(lambda x: parser.parse(x)))
df_trips["endTime"] = list(df_trips["endTime"].map(lambda x: parser.parse(x)))


# remove trips that are lower than the discretisation duration

low_duration_filter = df_trips.apply(lambda row: (row["endTime"] - row["startTime"]).total_seconds() , axis=1) <= (min_per_period) * 60
trips_with_low_duration = df_trips[low_duration_filter]
df_trips = df_trips[low_duration_filter == False]


# get time and project to today to form datetime
# time delta is only possible between two datetimes, so we just populate the date
# with todays date



#firstTime = min(df_trips["startTime"].map(lambda  x: datetime.datetime.combine(datetime.date.today(), x.time())))


#df_trips = df_trips.assign(startPeriod = list(df_trips["startTime"].map(
#    lambda x: int(math.floor(( datetime.datetime.combine(datetime.date.today(),x.time()) - firstTime).total_seconds() / 60.0 / min_per_period)))
#))

#df_trips = df_trips.assign(endPeriod = list(df_trips.apply(lambda row: int(math.floor(
#    # duration between start and end in periods
#    (( row["endTime"] - row["startTime"]).total_seconds() / 60.0 / min_per_period) + row["startPeriod"]
#)), axis=1)))




# start in period 6 and end in period 6 = 1 duration; Next can only start in 7


df_trips = df_trips.drop_duplicates("id")
def find_trip(i):
    if not i.strip() in df_trips.index:
        return None
    return df_trips.loc[i.strip()]

vehicle_with_trips = df_vehicles.assign(
    ftrips = df_vehicles["trips"].apply(lambda r:  list(filter(lambda l: l is not None,map(find_trip,r.strip('[ ]').split(',')))))
)

for idx,vehicle in vehicle_with_trips.iterrows():

    first_trip = vehicle["ftrips"][0]
    first_trip_start = first_trip["startTime"]
    # convert to period
    date_start = datetime.datetime.combine(first_trip_start.date(), datetime.time(0,0,0))

    for trip in vehicle["ftrips"]:
        df_trips.loc[trip["id"],"startPeriod"] = int(
            math.floor((trip["startTime"] - date_start).total_seconds() / 60.0 / min_per_period)
        )


df_trips = df_trips.assign(endPeriod = list(df_trips.apply(lambda row: int(math.floor(
    # duration between start and end in periods
    (( row["endTime"] - row["startTime"]).total_seconds() / 60.0 / min_per_period) + row["startPeriod"]
)), axis=1)))

invalid_trips = []
for idx,vehicle in vehicle_with_trips.iterrows():
    last_trip = None
    for ftrip in vehicle["ftrips"]:
        trip = df_trips.loc[ftrip["id"]]
        if trip["endPeriod"] < trip["startPeriod"]:
            print("Invalid Period ",trip["id"], file=sys.stderr)
            exit(-1)

        df_trips.loc[ftrip["id"], "inUse"] = True

        if last_trip is not None:
            if last_trip["endPeriod"] >= trip["startPeriod"]:

                #print("MUST ADJUST TRIP ", trip["id"], file=sys.stderr)
                #print("trip is",trip, file=sys.stderr)
                #print("last_trip is",last_trip, file=sys.stderr)

                desired_delta = (last_trip["endPeriod"] - trip["startPeriod"] + 1)
                possible_delta_self = (trip["endPeriod"]- trip["startPeriod"])

                if desired_delta <= possible_delta_self:
                    #print("\t Fixed by starting self by one later ", desired_delta, file=sys.stderr)
                    df_trips.loc[trip["id"],"startPeriod"] += desired_delta
                else:

                    #print("\t Fixed by shortening self by first lowering self by ", possible_delta_self, file=sys.stderr)
                    df_trips.loc[trip["id"], "startPeriod"] += possible_delta_self
                    desired_delta -= possible_delta_self
                    possible_delta_other = last_trip["endPeriod"] - last_trip["startPeriod"]

                    #print("\t then lowering previous (",last_trip["id"],") by", desired_delta, file=sys.stderr)


                    if(desired_delta <= possible_delta_other):
                        df_trips.loc[last_trip["id"], "endPeriod"] -= desired_delta
                    else:
                        print("COULD NOT FIX",last_trip["id"]," to ",trip["id"], file=sys.stderr)
                        invalid_trips.append(last_trip["id"])


            else:
                pass
                #print("TRIP ", trip["id"]," is OK", file=sys.stderr)


        last_trip = df_trips.loc[ftrip["id"]]

df_trips = df_trips[pd.notna(df_trips["inUse"])].drop(["inUse"],axis=1)
df_trips["startPeriod"] = df_trips["startPeriod"].astype(int)
df_trips["endPeriod"] = df_trips["endPeriod"].astype(int)
df_trips.to_csv(output,index=False)


df_vehicles["trips"] = vehicle_with_trips["ftrips"].apply(lambda r : "[ " + ",".join(list(map(lambda t: t["id"], r))) + " ]")

if len(invalid_trips) > 0:
    only_valid = df_vehicles[~df_vehicles.trips.str.contains("|".join(invalid_trips))]
else:
    only_valid = df_vehicles

only_valid.to_csv(vehicle_output_file,index=False)

print(output.getvalue())
