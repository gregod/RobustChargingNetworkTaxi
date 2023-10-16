"""

Arguments: vehicle_file, trips_file, dist_file, num_vehicles, seed, vehicle_output, trip_output
"""

# %%
import sys
import toml
import math
import numpy as np

battery_file = "/home/gregor/Code/et/pipeline/input_data/battery_1.toml"
d_bat = 1.01
d_charger = 1.10
d_final = 1.05

_, battery_file, d_bat, d_charger, d_final = sys.argv

parsed_toml = toml.loads(open(battery_file, 'r').read())

if ("time_to_soc" in parsed_toml):
    print("Must before time_to_soc processing",file=sys.stderr)
    exit(1)


d_bat = float(d_bat)
d_charger = float(d_charger)
d_final = float(d_final)

parsed_toml["battery_size"]  = round(parsed_toml["battery_size"] * d_bat,2)
parsed_toml["range_in_km"] = int(round(parsed_toml["range_in_km"] * d_bat,2))
parsed_toml["charging_speed"] =  round(parsed_toml["charging_speed"] * d_charger,2)
parsed_toml["SOC_final"] += (d_final-1.0) # SOC_final is allready in pct, so transform factor in pct points


print(toml.dumps(parsed_toml))
# %%



