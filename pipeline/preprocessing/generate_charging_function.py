"""

Arguments: vehicle_file, trips_file, dist_file, num_vehicles, seed, vehicle_output, trip_output
"""

# %%
import sys
import toml
import math
import numpy as np

_, battery_file = sys.argv
#battery_file = "/home/gregor/Code/et/pipeline/input_data/battery_2.toml"
parsed_toml = toml.loads(open(battery_file, 'r').read())

U_n = 3.6
U_ls = 4.2
SOC_min = parsed_toml["SOC_min"]
SOC_max = parsed_toml["SOC_max"]
P_l = parsed_toml["charging_speed"]
n_l = 0.9
E_b = parsed_toml["battery_size"]

s = -0.008 * P_l + 0.83
I_ls = 0.006 * P_l + 0.008

P_ls = U_ls/U_n * I_ls * E_b
k_l = (1-s)/math.log(P_l/P_ls)

def P(SOCp):
    if SOCp < s:
        return P_l
    else:
        return P_l * math.exp((s - SOCp)/k_l)

def SOC(SOC_p,d_t):
    return SOC_p + ((P(SOC_p) * d_t) / E_b) * n_l

last_val= SOC_min
values = []
while True:
    if last_val > SOC_max:
        break
    cur_val = SOC(last_val,1/60)
    values.append(cur_val)
    last_val = cur_val

minutes = range(1,len(values)+1)
time_to_soc = np.polyfit(minutes,values,3)
soc_to_time = np.polyfit(values,minutes,3)

parsed_toml["time_to_soc"] = time_to_soc.tolist()
parsed_toml["soc_to_time"] = soc_to_time.tolist()


print(toml.dumps(parsed_toml))