# %%
import pandas as pd
import sys
# %%


tableMode = True

scenarioFeas = True

cross_feasibility_file =  sys.argv[1]
expected_num_vehicles_str  =  sys.argv[2]

if len(sys.argv) > 3:
 scenarioFeas = sys.argv[3] == "scenario"


expected_num_vehicles = int(expected_num_vehicles_str)



#cross_feasibility_file = "/home/gregor/Code/et/pipeline/"+"work/opt/robust/battery_1/circle_2_50_10/500/lowest_cross_feasibility_quorum:100_activate:1_benevolent:4"

# %%

df_full = pd.read_csv(cross_feasibility_file,sep="|",names=["trip_file","vehicle_file","status","status_code","inf","orig_vehicles"])
# %%


def chunk(seq, size):
    return (seq[pos:pos + size] for pos in range(0, len(seq), size))


for i,df in enumerate(chunk(df_full, 28)):


    number_of_infeasible_vehicles = expected_num_vehicles-(df["orig_vehicles"]-df["inf"])
    inf_vehicles = number_of_infeasible_vehicles / expected_num_vehicles
    stats = (inf_vehicles * 100).describe()
    scenario_percent_inf = ((df.status != "FEASIBLE").sum()/df.status.count())*100

    if scenarioFeas:
        if tableMode:
            print("$\mathcal{Z}^%d$                 &  %.2f  \\\\" %(i+1,100-scenario_percent_inf))
        else:
            print("sfeas:",scenario_percent_inf)
            
    else:
        if tableMode:
            print("$\mathcal{Z}^%d$                 &  %.2f      &  %.2f   \\\\" %(i+1,100 - stats["mean"],100 - stats["max"]))
        else:
            print(df.describe())
            