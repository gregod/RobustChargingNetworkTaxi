import pandas as pd
import re
import sys
import csv
import io


_, xtest_file, orig_vehicle_file,orig_trip_file, new_trip_file = sys.argv

df = pd.read_csv(xtest_file, delimiter="|",names=("vehicle_file","trip_file","status","details"))
not_feasible = df[df["status"] == "NOT_FEASIBLE"]


merged_df_vehicles = pd.read_csv(orig_vehicle_file)
df_merged_trips = pd.read_csv(orig_trip_file)

for k,el in not_feasible.iterrows():
    if (not el["details"].startswith("VehiclesInfeasible")):
        print("Vehcile does not start with VehiclesInfeasible", file=sys.stderr)
        exit(-1)

    indexes_of_infeasible = [ int(v) for v in el["details"][el["details"].find("[")+1:-2].split(",")]

    vehicle = pd.read_csv(el["vehicle_file"])
    vehicles = vehicle.iloc[indexes_of_infeasible]


    # add infeasible vehicles to new set.
    merged_df_vehicles = merged_df_vehicles.append(vehicles)




    # find trips of new vehicles

    for k,vehicle in vehicles.iterrows():

        v_trip_file = el["trip_file"]
        df_trips = pd.read_csv(v_trip_file).drop_duplicates("id")
        df_trips.index = df_trips["id"]

        vehicle_with_trips = vehicle["trips"].strip('[ ]').split(',')


        df_merged_trips = df_merged_trips.append(df_trips.loc[vehicle_with_trips])





merged_df_vehicles.index = range(0,len(merged_df_vehicles))

merged_df_vehicles.to_csv(path_or_buf=sys.stdout, quoting=csv.QUOTE_MINIMAL, index=False)
df_merged_trips.to_csv(path_or_buf=new_trip_file,quoting=csv.QUOTE_NONNUMERIC, index=False)