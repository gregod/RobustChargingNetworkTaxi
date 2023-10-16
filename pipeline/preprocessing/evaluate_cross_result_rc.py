# %%
import pandas as pd
import sys
import re
# %%

_, cross_feasibility_file  = sys.argv

#cross_feasibility_file = "/home/gregor/Code/et/pipeline/"+"work/opt/realcase/robust/dbat:1.00_dcha:1.00_dfin:1.15/lowest_cross_feasibility_quorum:100_activate:1_benevolent:5_iis:true"

# %%

df = pd.read_csv(cross_feasibility_file,sep="|",names=["trip_file","vehicle_file","status","status_code","inf","orig_vehicles"])
# %%

reg = "(.*)/preprocessed/realcase/vehicles.base.dbat:(\d.\d\d)_dcha:(\d.\d\d)_dfin:(\d.\d\d).(.*\d).csv.gz"
def get_orig_vehicle_count(file_name):
    gr = re.match(reg,file_name).groups()
    fn = f"{gr[0]}/preprocessed/realcase/vehicles.feasible.{gr[4]}.csv.gz"
    return pd.read_csv(fn).index.size


df["realcase_vehicles"] = df["trip_file"].apply(get_orig_vehicle_count)

# %%


# a = rc_v
# b = org_v
# c = inf

# feasible = org_v - inf
# inf = 


vehicle_feasible = (df["orig_vehicles"] - df["inf"])
vehicle_percent_inf =  1-(vehicle_feasible.sum() / df["realcase_vehicles"].sum())


scenario_percent_inf = (df.status != "FEASIBLE").sum()/df.status.count()

print(str(vehicle_percent_inf) + "|" + str(scenario_percent_inf))
# %%
