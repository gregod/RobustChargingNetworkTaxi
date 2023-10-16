# %%
import pandas as pd
import sys
# %%

_, cross_feasibility_file, expected_num_vehicles_str  = sys.argv

expected_num_vehicles = int(expected_num_vehicles_str)
#cross_feasibility_file = "/home/gregor/Code/et/pipeline/"+"work/opt/robust/battery_1/circle_2_50_10/500/lowest_cross_feasibility_quorum:100_activate:1_benevolent:4"

# %%

df = pd.read_csv(cross_feasibility_file,sep="|",names=["trip_file","vehicle_file","status","status_code","inf","orig_vehicles"])
# %%

number_of_infeasible_vehicles = expected_num_vehicles-(df["orig_vehicles"]-df["inf"])

vehicle_percent_mean_inf = (number_of_infeasible_vehicles / expected_num_vehicles).mean()
vehicle_percent_max_inf = (number_of_infeasible_vehicles / expected_num_vehicles).max()
scenario_percent_inf = (df.status != "FEASIBLE").sum()/df.status.count()

print(str(vehicle_percent_mean_inf) + "|" + str(scenario_percent_inf) + "|" + str(vehicle_percent_max_inf))