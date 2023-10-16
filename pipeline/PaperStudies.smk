


full_parteto_test_circle_strategies = [ "old_f60", "kde_60", "threecircle_1-2_10-10-40", "threecircle_1-2_20-10-30", "threecircle_1-2_30-10-20", "threecircle_1-2_40-10-10", "threecircle_1-2_10-20-30", "threecircle_1-2_20-20-20", "threecircle_1-2_30-20-10", "threecircle_1-2_40-10-10", "threecircle_1-3_10-10-40", "threecircle_1-3_20-10-30", "threecircle_1-3_30-10-20", "threecircle_1-3_40-10-10", "threecircle_1-3_10-20-30", "threecircle_1-3_20-20-20", "threecircle_1-3_30-20-10", "threecircle_1-3_40-10-10", "threecircle_1-4_10-10-40", "threecircle_1-4_20-10-30", "threecircle_1-4_30-10-20", "threecircle_1-4_40-10-10", "threecircle_1-4_10-20-30", "threecircle_1-4_20-20-20", "threecircle_1-4_30-20-10", "threecircle_1-4_40-10-10", "threecircle_2-3_10-10-40", "threecircle_2-3_20-10-30", "threecircle_2-3_30-10-20", "threecircle_2-3_40-10-10", "threecircle_2-3_10-20-30", "threecircle_2-3_20-20-20", "threecircle_2-3_30-20-10", "threecircle_2-3_40-10-10", "threecircle_2-4_10-10-40", "threecircle_2-4_20-10-30", "threecircle_2-4_30-10-20", "threecircle_2-4_40-10-10", "threecircle_2-4_10-20-30", "threecircle_2-4_20-20-20", "threecircle_2-4_30-20-10", "threecircle_2-4_40-10-10", "threecircle_3-4_10-10-40", "threecircle_3-4_20-10-30", "threecircle_3-4_30-10-20", "threecircle_3-4_40-10-10", "threecircle_3-4_10-20-30", "threecircle_3-4_20-20-20", "threecircle_3-4_30-20-10", "threecircle_3-4_40-10-10", "circle_2_10_50", "circle_2_20_40", "circle_2_30_30", "circle_2_40_20", "circle_2_50_10", "circle_3_10_50", "circle_3_20_40", "circle_3_30_30", "circle_3_40_20", "circle_3_50_10", "circle_4_10_50", "circle_4_20_40", "circle_4_30_30", "circle_4_40_20", "circle_5_10_50", "circle_5_20_40", "circle_5_30_30", "circle_5_50_10" ]
configfile: "config.yaml"

OUTPUT_PREFIX = config["output_prefix"]

include: "common.smk"
include: "BuildBinaries.smk"
include: "Preprocessing.smk"
include: "PerformanceScenarios.smk"
include: "RealCase.smk"
include: "NewIntegrated.smk"


RC_SEN_RANGE=["0.85","0.90","0.95","1.00","1.05","1.10","1.15"]
RC_DAY_RANGE=["{:02}".format(i) for i in range(1,31 +1) ]


rule robust_methods:
    input:
         ## PERFORMANCE SCENARIOS
          expand(OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/battery_1/tol00/f{NUM_SITES}/{NUM_VEHICLES}/opt_log",
                     SEED = [1,2,3],
                     TYPE_GROUP = [0,1,2,3,4,5,6],
                     NUM_SITES =[30,35,40,45,50,55,60,65,70],
                     NUM_VEHICLES = list(map(lambda x : x * 100,[1,2,3,4,5,6,7,8,9,10]))
                 ),
          expand(OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/battery_1/tol00/f{NUM_SITES}/{NUM_VEHICLES}/opt_log",
                     SEED = [1,2,3],
                     TYPE_GROUP = [0,1,2,3,4,5,6],
                     NUM_SITES =[60],
                     NUM_VEHICLES = list(map(lambda x : x * 100,[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15]))
                 ),

         ## ROBUST SCENARIOS
             #MEDIAN LOWEST
             expand(OUTPUT_PREFIX + "/opt/robust/battery_1/old_f60/{NUM_VEHICLES}/{TYPE}_seed_opt_log_filename_tol00",TYPE=["median","lowest"],NUM_VEHICLES = [500,1000]),
             expand(OUTPUT_PREFIX + "/opt/robust/battery_1/old_f60/{NUM_VEHICLES}/{TYPE}_percent_feasible_tol00",TYPE=["median","lowest"],NUM_VEHICLES = [500,1000]),
             #ASA
             expand(OUTPUT_PREFIX +  "/opt/robust/battery_1/old_f60/{NUM_VEHICLES}/{TYPE}_{OUTPUT_FILE}_quorum:100_activate:1_benevolent:{BENEVOLENT}{ON}_iis:{IIS}",
                ON=["_on:vehicles",""],
                TYPE=["median","lowest"],
                BENEVOLENT=[1,2,3,4,5,6,7,8,9,10],
                IIS=["true","false"],
                OUTPUT_FILE=["opt_log","percent_feasible"],
                 NUM_VEHICLES = [500,1000]
                ),
             #FULL
             expand(OUTPUT_PREFIX +  "/opt/robust/battery_1/old_f60/{NUM_VEHICLES}/full_{OUTPUT_FILE}_quorum:{QUORUM}",
                 QUORUM=[100,95,90,75,50],
                 OUTPUT_FILE=["opt_log","percent_feasible"],
                 NUM_VEHICLES = [500,1000]
                ),
             #ISA- active_sites
                expand(OUTPUT_PREFIX +  "/opt/{SEED}/group_{TYPE_GROUP}/battery_1/tol00/old_f{NUM_SITES}/{NUM_VEHICLES}/active_sites.csv",
                              SEED = [1,2,3],
                              TYPE_GROUP = [0,1,2,3,4,5,6],
                              NUM_VEHICLES = [500,1000],
                              NUM_SITES =[60],
                             ),
        ## Robust Pareto Full (with circle)
             expand(OUTPUT_PREFIX +  "/opt/robust/{BATTERY}/{STRATEGY}/{NUM_VEHICLES}/full_{OUTPUT_FILE}_quorum:100",
                BATTERY=["battery_1"],
                STRATEGY = full_parteto_test_circle_strategies,
                NUM_VEHICLES = [500],
                OUTPUT_FILE=["opt_log","percent_feasible"],
                ),
        # Real Case

         
         # full opt
             expand(OUTPUT_PREFIX+  "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/full_{filetype}_quorum:{QUORUM_ACCEPT}",
                DBAT=RC_SEN_RANGE,DCHAR="1.00",DFINAL="1.00",QUORUM_ACCEPT="100",filetype=["opt_log","percent_feasible"]
             ),
             expand(OUTPUT_PREFIX+  "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/full_{filetype}_quorum:{QUORUM_ACCEPT}",
                DBAT="1.00",DCHAR=RC_SEN_RANGE,DFINAL="1.00",QUORUM_ACCEPT="100",filetype=["opt_log","percent_feasible"]
             ),
             expand(OUTPUT_PREFIX+  "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/full_{filetype}_quorum:{QUORUM_ACCEPT}",
                DBAT="1.00",DCHAR="1.00",DFINAL=RC_SEN_RANGE,QUORUM_ACCEPT="100",filetype=["opt_log","percent_feasible"]
             ),
         # deterministic
             expand(OUTPUT_PREFIX+  "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{filetype}",
                DAY=RC_DAY_RANGE,DBAT=RC_SEN_RANGE,DCHAR="1.00",DFINAL="1.00",filetype=["opt_log","percent_feasible"]
             ),
             expand(OUTPUT_PREFIX+  "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{filetype}",
                DAY=RC_DAY_RANGE,DBAT="1.00",DCHAR=RC_SEN_RANGE,DFINAL="1.00",filetype=["opt_log","percent_feasible"]
             ),
             expand(OUTPUT_PREFIX+  "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{filetype}",
                DAY=RC_DAY_RANGE,DBAT="1.00",DCHAR="1.00",DFINAL=RC_SEN_RANGE,filetype=["opt_log","percent_feasible"]
             ),
            # deterministic median
             expand(OUTPUT_PREFIX+  "/opt/realcase/median{SEED}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{filetype}",
                SEED=[1],DBAT=RC_SEN_RANGE,DCHAR="1.00",DFINAL="1.00",filetype=["opt_log","percent_feasible"]
             ),
             expand(OUTPUT_PREFIX+  "/opt/realcase/median{SEED}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{filetype}",
                SEED=[1],DBAT="1.00",DCHAR=RC_SEN_RANGE,DFINAL="1.00",filetype=["opt_log","percent_feasible"]
             ),
             expand(OUTPUT_PREFIX+  "/opt/realcase/median{SEED}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{filetype}",
                SEED=[1],DBAT="1.00",DCHAR="1.00",DFINAL=RC_SEN_RANGE,filetype=["opt_log","percent_feasible"]
             ),
         # benevvolent sensitivity
             expand(OUTPUT_PREFIX+  "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{TYPE}_{OUTPUT_FILE}_quorum:100_activate:1_benevolent:{BENEVOLENT}_iis:{IIS}",
                DAY=RC_DAY_RANGE,DBAT=RC_SEN_RANGE,DCHAR="1.00",DFINAL="1.00",
                TYPE=["lowest"],
                BENEVOLENT=[5,10],
                IIS=["true"],
                OUTPUT_FILE=["opt_log","percent_feasible"],
             ),
             expand(OUTPUT_PREFIX+  "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{TYPE}_{OUTPUT_FILE}_quorum:100_activate:1_benevolent:{BENEVOLENT}_iis:{IIS}",
                DAY=RC_DAY_RANGE,DBAT="1.00",DCHAR=RC_SEN_RANGE,DFINAL="1.00",
                TYPE=["lowest"],
                BENEVOLENT=[5,10],
                IIS=["true"],
                OUTPUT_FILE=["opt_log","percent_feasible"],
             ),
             expand(OUTPUT_PREFIX+  "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{TYPE}_{OUTPUT_FILE}_quorum:100_activate:1_benevolent:{BENEVOLENT}_iis:{IIS}",
                DAY=RC_DAY_RANGE,DBAT="1.00",DCHAR="1.00",DFINAL=RC_SEN_RANGE,
                TYPE=["lowest"],
                BENEVOLENT=[5,10],
                IIS=["true"],
                OUTPUT_FILE=["opt_log","percent_feasible"],
             ),
         # benevolent parameter search
             expand(OUTPUT_PREFIX+  "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{TYPE}_{OUTPUT_FILE}_quorum:100_activate:1_benevolent:{BENEVOLENT}_iis:{IIS}",
                DAY=RC_DAY_RANGE,DBAT="1.00",DCHAR="1.00",DFINAL="1.00",
                TYPE=["lowest"],
                BENEVOLENT=[1,2,3,4,5,6,7,8,9,10],
                IIS=["true"],
                OUTPUT_FILE=["opt_log","percent_feasible"],
             )

rule server_preprocessed:
    output:
       OUTPUT_PREFIX + "/opt/server/flag1"
    input:
        # for performance scenarios
        expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/f{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.{FILE_TYPE}.csv.gz",
                     SEED = [1,2,3],
                     TYPE_GROUP = [0,1,2,3,4,5,6],
                     NUM_SITES =[30,35,40,45,50,55,60,65,70],
                     BATTERY=["battery_1"],
                     NUM_VEHICLES = list(map(lambda x : x * 100,[1,2,3,4,5,6,7,8,9,10])),
                     FILE_TYPE=["vehicles","trips"]
        ),
        expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/f{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.{FILE_TYPE}.csv.gz",
                     SEED = [1,2,3],
                     TYPE_GROUP = [0,1,2,3,4,5,6],
                     NUM_SITES =[60],
                     BATTERY=["battery_1"],
                     NUM_VEHICLES = list(map(lambda x : x * 100,[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15])),
                     FILE_TYPE=["vehicles","trips"]
        ),

         # for robust / new integrated
        expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_VEHICLES}/old_f{NUM_SITES}/{BATTERY}.feasible.vehicles.csv.gz",
             SEED = [1,2,3],
             TYPE_GROUP = [0,1,2,3,4,5,6],
             NUM_SITES =[60],
             BATTERY=["battery_1"],
             NUM_VEHICLES = list(map(lambda x : x * 100,[5,10]))
        ),
        expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_VEHICLES}/old_f{NUM_SITES}/fixed.trips.csv.gz",
             NUM_SITES =[60],
             SEED = [1,2,3],
             TYPE_GROUP = [0,1,2,3,4,5,6],
             NUM_VEHICLES = list(map(lambda x : x * 100,[5,10]))
          ),
        # for robust site selection process
        expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_VEHICLES}/{STRATEGY}/fixed.trips.csv.gz",
             NUM_SITES =[60],
             STRATEGY = full_parteto_test_circle_strategies,
             SEED = [1,2,3],
             TYPE_GROUP = [0,1,2,3,4,5,6],
             NUM_VEHICLES = [500]
          ),
        expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_VEHICLES}/{STRATEGY}/{BATTERY}.feasible.vehicles.csv.gz",
             NUM_SITES =[60],
             STRATEGY = full_parteto_test_circle_strategies,
             SEED = [1,2,3],
             TYPE_GROUP = [0,1,2,3,4,5,6],
             BATTERY=["battery_1"],
             NUM_VEHICLES = [500]
          ),

        expand(OUTPUT_PREFIX+  "/preprocessed/realcase/vehicles.base.dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.{DAY}.csv.gz", DAY=RC_DAY_RANGE,DBAT=RC_SEN_RANGE,DCHAR="1.00",DFINAL="1.00" ),
        expand(OUTPUT_PREFIX+  "/preprocessed/realcase/vehicles.base.dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.{DAY}.csv.gz",DAY=RC_DAY_RANGE,DBAT="1.00",DCHAR=RC_SEN_RANGE,DFINAL="1.00"),
        expand(OUTPUT_PREFIX+  "/preprocessed/realcase/vehicles.base.dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.{DAY}.csv.gz", DAY=RC_DAY_RANGE,DBAT="1.00",DCHAR="1.00",DFINAL=RC_SEN_RANGE),
        expand(OUTPUT_PREFIX+  "/preprocessed/realcase/vehicles.base.dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.median{SEED}.csv.gz", SEED=[1,2,3,4,6,7,8,9,10],DBAT=RC_SEN_RANGE,DCHAR="1.00",DFINAL="1.00" ),
        expand(OUTPUT_PREFIX+  "/preprocessed/realcase/vehicles.base.dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.median{SEED}.csv.gz",SEED=[1,2,3,4,6,7,8,9,10],DBAT="1.00",DCHAR=RC_SEN_RANGE,DFINAL="1.00"),
        expand(OUTPUT_PREFIX+  "/preprocessed/realcase/vehicles.base.dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.median{SEED}.csv.gz", SEED=[1,2,3,4,6,7,8,9,10],DBAT="1.00",DCHAR="1.00",DFINAL=RC_SEN_RANGE),
        expand(OUTPUT_PREFIX+  "/preprocessed/realcase/trips.all.{DAY}.csv.gz", DAY=RC_DAY_RANGE,DBAT=RC_SEN_RANGE,DCHAR="1.00",DFINAL="1.00" ),
        expand(OUTPUT_PREFIX+  "/preprocessed/realcase/trips.all.{DAY}.csv.gz",DAY=RC_DAY_RANGE,DBAT="1.00",DCHAR=RC_SEN_RANGE,DFINAL="1.00"),
        expand(OUTPUT_PREFIX+  "/preprocessed/realcase/trips.all.{DAY}.csv.gz", DAY=RC_DAY_RANGE,DBAT="1.00",DCHAR="1.00",DFINAL=RC_SEN_RANGE),
        expand(OUTPUT_PREFIX+  "/preprocessed/realcase/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml", DAY=RC_DAY_RANGE,DBAT=RC_SEN_RANGE,DCHAR="1.00",DFINAL="1.00" ),
        expand(OUTPUT_PREFIX+  "/preprocessed/realcase/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml",DAY=RC_DAY_RANGE,DBAT="1.00",DCHAR=RC_SEN_RANGE,DFINAL="1.00"),
        expand(OUTPUT_PREFIX+  "/preprocessed/realcase/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml", DAY=RC_DAY_RANGE,DBAT="1.00",DCHAR="1.00",DFINAL=RC_SEN_RANGE),
        

    shell:
        "touch {output}"
