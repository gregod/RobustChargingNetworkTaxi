include: "common.smk"

TC_BASE_BATTERY = "battery_1"
DAY_RANGE=["{:02}".format(i) for i in range(1,31 +1) ]

rob_seeds_strat=config["robust"]["seeds"]
rob_groups_strat=config["robust"]["groups"]

rule tc_calculate_battery:
    group: "preprocessing"
    conda:
        "environment.yaml"
    resources:
        runtime=3600
    input:
        battery="input_data/" + TC_BASE_BATTERY +".toml",
        script="preprocessing/create_rc_battery.py"
    output:
        battery=OUTPUT_PREFIX + "/preprocessed/raw_dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml"
    shell:
        "python {input.script} {input.battery} {wildcards.DBAT} {wildcards.DCHAR} {wildcards.DFINAL} > {output.battery}"

rule tc_apply_battery_function:
    group: "preprocessing"
    conda:
        "environment.yaml"
    resources:
        runtime=3600
    input:
        battery=OUTPUT_PREFIX + "/preprocessed/raw_dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml",
        script="preprocessing/generate_charging_function.py"
    output:
        battery=OUTPUT_PREFIX + "/preprocessed/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml"
    shell:
        "python {input.script} {input.battery} > {output.battery}"


rule ensure_feasible_for_tc_base_config:
    group: "preprocessing"
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/capped_8.vehicles.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/final.trips.csv.gz",
        sites=OUTPUT_PREFIX + "/preprocessed/nocost_{NUM_SITES}.sites.csv",
        battery=OUTPUT_PREFIX +"/preprocessed/dbat:2.00_dcha:2.00_dfin:1.00.toml",
        binary=OUTPUT_PREFIX + "/binaries/remove_infeasible"
    output:
        vehicles=OUTPUT_PREFIX + "/preprocessed/techcase/group_{TYPE_GROUP}/{NUM_SITES}/base.feasible.vehicles.csv.gz"
    shell:
        "{input.binary} --vehicles {input.vehicles} --trips {input.trips} --sites {input.sites} --battery {input.battery} | gzip  > {output.vehicles}"

rule generate_sample_from_tc_base_group:
    group: "preprocessing"
    conda:
        "environment.yaml"
    input:
        script="preprocessing/generate_sample.py",
        vehicles=OUTPUT_PREFIX + "/preprocessed/techcase/group_{TYPE_GROUP}/{NUM_SITES}/base.feasible.vehicles.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/final.trips.csv.gz",
        dist=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/distribution.txt",
    output:
        vehicles=OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/base.vehicles.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/base.sampled.trips.csv.gz"
    shell:
        "python {input.script} {input.vehicles} {input.trips} {input.dist} {wildcards.NUM_VEHICLES} {wildcards.SEED} {output.vehicles} {output.trips}"



rule tc_base_resample_capacity_infeasible_vehicles:
    group: "preprocessing"
    conda:
        "environment.yaml"
    resources:
             runtime=3600
    input:
         vehicles=OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/base.vehicles.csv.gz",
         trips=OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/base.sampled.trips.csv.gz",

         presample_vehicles=OUTPUT_PREFIX + "/preprocessed/techcase/group_{TYPE_GROUP}/{NUM_SITES}/base.feasible.vehicles.csv.gz",
         presample_trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/final.trips.csv.gz",

         # retain only the digits for nocost!
         sites=OUTPUT_PREFIX + "/preprocessed/nocost_{NUM_SITES}.sites.csv",
         battery=OUTPUT_PREFIX +"/preprocessed/dbat:2.00_dcha:2.00_dfin:1.00.toml",
         binary=OUTPUT_PREFIX + "/binaries/check_feasibility",
         script= "preprocessing/resample_till_feasible.py"
    output:
         vehicles=OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/base.final.vehicles.csv.gz",
         trips=OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/base.final.trips.csv.gz"
    shell:
        "python {input.script} {input.vehicles} {input.trips} {input.binary} {input.presample_vehicles} {input.presample_trips} {input.sites} {input.battery} {wildcards.SEED} {output.vehicles} {output.trips}"





rule remove_infeasible_vehicles_for_battery_from_base:
    group: "preprocessing"
    conda:
        "environment.yaml"
    resources:
             runtime=3600
    input:
         trips= OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/base.final.trips.csv.gz",
         vehicles=OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/base.final.vehicles.csv.gz",
         # retain only the digits for nocost!
         sites=OUTPUT_PREFIX + "/preprocessed/nocost_{NUM_SITES}.sites.csv",
         battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
         binary=OUTPUT_PREFIX + "/binaries/check_feasibility",
         script="preprocessing/remove_infeasible_with_capacity.py"
    output:
          vehicles=OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.vehicles.csv.gz",
    shell:
         "python {input.script} {input.vehicles} {input.trips} {input.binary} {input.sites} {input.battery} {output.vehicles}"


# SEEDS

rule run_opt_on_tech_group:
    resources:
             runtime=lambda wildcards, attempt: 3 * (60 * 60) if int(wildcards.INT_NUM_SITES) <= 30 else 6 * (60*60), mem_mb=4000
    group: "solving_bucket"
    threads: 1
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.vehicles.csv.gz",
        sites=OUTPUT_PREFIX + "/preprocessed/{INT_NUM_SITES}{SUFFIX_DASH}.sites.csv",
        trips = OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/base.final.trips.csv.gz",
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        binary=OUTPUT_PREFIX + "/binaries/benders"
    output:
        stdout=OUTPUT_PREFIX + "/opt/techcase/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/opt_log",
        charge_process=OUTPUT_PREFIX + "/opt/techcase/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/opt_chargeprocess",
        cuts=OUTPUT_PREFIX + "/opt/techcase/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/opt_cuts"
    log:
        trace=OUTPUT_PREFIX + "/opt/techcase/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/opt_log_trace.bin"
    shell:
        "{input.binary}  --vehicles {input.vehicles} --trips {input.trips} --cuts_output {output.cuts}  --sites {input.sites} --battery {input.battery} --sites_min=1 --percent_infeasible_allowed 0.{wildcards.TOLERANCE} --charge_processes_file {output.charge_process} --hawktracer_output {log.trace} > {output.stdout}"

rule extract_site_csv_techcase:
    group: "solving_bucket"
    input:
         script="preprocessing/extract_site_results.py",
         sites=OUTPUT_PREFIX + "/preprocessed/{INT_NUM_SITES}{SUFFIX_DASH}.sites.csv",
         optlog=OUTPUT_PREFIX + "/opt/techcase/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/opt_log"
    output:
          OUTPUT_PREFIX + "/opt/techcase/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/active_sites"
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.optlog} {input.sites} > {output}"





# ROBUST


rule techcase_run_FSA_robust:
    group: "solving_bucket"
    threads: 1
    input:
        sites=OUTPUT_PREFIX + "/preprocessed/{INT_NUM_SITES}.sites.csv",

        vehicles=expand(OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{GROUP}/{{INT_NUM_SITES}}/{{NUM_VEHICLES}}/dbat:{{DBAT}}_dcha:{{DCHAR}}_dfin:{{DFINAL}}.final.vehicles.csv.gz",
            DAY=DAY_RANGE,
            SEED=rob_seeds_strat,
            GROUP = rob_groups_strat,
        ),
        
        trips=expand(OUTPUT_PREFIX  + "/preprocessed/techcase/{SEED}/group_{GROUP}/{{INT_NUM_SITES}}/{{NUM_VEHICLES}}/base.final.trips.csv.gz",
            SEED=rob_seeds_strat,
            GROUP = rob_groups_strat,
        ),
        battery=OUTPUT_PREFIX +"/preprocessed/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml",
        binary=OUTPUT_PREFIX + "/binaries/robust2"
    output:
        stdout=OUTPUT_PREFIX + "/opt/techcase/robust/{INT_NUM_SITES}/{NUM_VEHICLES}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/full_opt_log_quorum:{QUORUM_ACCEPT}",
    shell:
        "{input.binary} --total_num_vehicles {wildcards.NUM_VEHICLES} --vehicles  {input.vehicles} --trips  {input.trips}  --sites {input.sites} --battery {input.battery} --quorum_accept_percent={wildcards.QUORUM_ACCEPT} --activate_all > {output.stdout}"

rule run_robust2_tc:
    group: "solving_bucket"
    conda:
        "environment.yaml"
    threads: 1
    input:
        sites=OUTPUT_PREFIX + "/preprocessed/{INT_NUM_SITES}.sites.csv",
        
        seed_vehicle_opt_log= OUTPUT_PREFIX + "/opt/techcase/robust/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/{BATTERY}/{SEED_TYPE}_seed_opt_log_filename_tol00",
        vehicles=expand(OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{GROUP}/{{INT_NUM_SITES}}{{SUFFIX_DASH}}/{{NUM_VEHICLES}}/{{BATTERY}}.final.vehicles.csv.gz",
            SEED=rob_seeds_strat,
            GROUP = rob_groups_strat,
        ),
        trips=expand(OUTPUT_PREFIX  + "/preprocessed/techcase/{SEED}/group_{GROUP}/{{INT_NUM_SITES}}{{SUFFIX_DASH}}/{{NUM_VEHICLES}}/base.final.trips.csv.gz",
            SEED=rob_seeds_strat,
            GROUP = rob_groups_strat,
        ),

        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        opt_log_to_vehicles="preprocessing/opt_log_to_vehicles_tc.py",
        opt_log_to_trips="preprocessing/opt_log_to_trips_tc.py",
        opt_log_to_cuts="preprocessing/opt_log_to_cuts.py",
        binary=OUTPUT_PREFIX + "/binaries/robust2"

    output:
        stdout=OUTPUT_PREFIX + "/opt/techcase/robust/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/{BATTERY}/{SEED_TYPE}_opt_log_quorum:{QUORUM_ACCEPT}_activate:{MAX_ACTIVATE}_benevolent:{BENEVOLENT}_iis:{IIS}"
    shell:
        "{input.binary}  --vehicles $(python {input.opt_log_to_vehicles} $(cat {input.seed_vehicle_opt_log})) {input.vehicles} --trips $(python {input.opt_log_to_trips} $(cat {input.seed_vehicle_opt_log})) {input.trips}  --sites {input.sites} --battery {input.battery} --cuts_input  $(python {input.opt_log_to_cuts} $(cat {input.seed_vehicle_opt_log}))   --max_activate_per_generation={wildcards.MAX_ACTIVATE} --quorum_accept_percent={wildcards.QUORUM_ACCEPT} --activate_iis={wildcards.IIS} --benevolent_accept_percent={wildcards.BENEVOLENT}  > {output.stdout}"




rule extract_site_csv_techcase_robust:
    group: "solving_bucket"
    input:
         script="preprocessing/extract_site_results.py",
         sites=OUTPUT_PREFIX + "/preprocessed/{INT_NUM_SITES}{SUFFIX_DASH}.sites.csv",
         optlog=OUTPUT_PREFIX + "/opt/techcase/robust/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{PREFIX}_opt_log_{SUFFIX}",
    output:
         OUTPUT_PREFIX + "/opt/techcase/robust/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{PREFIX}_active_sites_{SUFFIX}",
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.optlog} {input.sites} > {output}"



rule check_cross_feasibility_of_robust_techcase:
    group : "cross_checking"
    resources:
             runtime=1800
    input:
        active_sites= OUTPUT_PREFIX + "/opt/techcase/robust/{NUM_SITES}/{NUM_VEHICLES}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{PREFIX}_active_sites_{SUFFIX}",
        vehicles=expand(
            OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{GROUP}/{{NUM_SITES}}/{{NUM_VEHICLES}}/base.vehicles.csv.gz",  
            SEED=rob_seeds_strat,
            GROUP = rob_groups_strat,
        ),
        trips=expand(
            OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{GROUP}/{{NUM_SITES}}/{{NUM_VEHICLES}}/base.final.trips.csv.gz",
              SEED=rob_seeds_strat,
            GROUP = rob_groups_strat,
        ),
        battery=OUTPUT_PREFIX +"/preprocessed/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml",
        binary=OUTPUT_PREFIX + "/binaries/check_cross_feasibility"
    output:
          OUTPUT_PREFIX + "/opt/techcase/robust/{NUM_SITES}/{NUM_VEHICLES}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{PREFIX}_cross_feasibility_{SUFFIX}"
    shell:
         "{input.binary} --percent_infeasible_allowed 0.00  --sites {input.active_sites} --battery {input.battery}  --trips {input.trips} --vehicles {input.vehicles} | grep -v '^Academic license' > {output}"


rule evaluate_cross_feasibility_of_robust_techcase:
    group : "cross_checking"
    conda:
        "environment.yaml"
    resources:
             runtime=1800
    input:
        cross_file=OUTPUT_PREFIX + "/opt/techcase/robust/{NUM_SITES}/{NUM_VEHICLES}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{PREFIX}_cross_feasibility_{SUFFIX}",
        script="preprocessing/evaluate_cross_result_tc.py"
    output:
        cross_file=OUTPUT_PREFIX + "/opt/techcase/robust/{NUM_SITES}/{NUM_VEHICLES}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{PREFIX}_percent_feasible_{SUFFIX}"
    shell:
        "python {input.script}  {input.cross_file} > {output}"
        




rule generate_seed_median_robust_opt_log_file_name_tc:
    conda:
        "environment.yaml"
    input:
        scriptE="preprocessing/find_median_opt_log.py",
        optLogs=expand(OUTPUT_PREFIX + "/opt/techcase/{SEED}/group_{GROUP}/{{BATTERY}}/tol{{TOLERANCE}}/{{INT_NUM_SITES}}{{SUFFIX_DASH}}/{{NUM_VEHICLES}}/opt_log",
            SEED=rob_seeds_strat,
            GROUP = rob_groups_strat,
        )
    output:
         OUTPUT_PREFIX + "/opt/techcase/robust/{{INT_NUM_SITES}}{{SUFFIX_DASH}}/{NUM_VEHICLES}/{BATTERY}/median_seed_opt_log_filename_tol{TOLERANCE}"
    shell:
        "python {input.scriptE} {input.optLogs} > {output}"


rule generate_seed_lowest_robust_opt_log_file_name_tc:
    conda:
        "environment.yaml"
    input:
        scriptE="preprocessing/find_lowest_opt_log.py",
        optLogs=expand(OUTPUT_PREFIX + "/opt/techcase/{SEED}/group_{GROUP}/{{BATTERY}}/tol{{TOLERANCE}}/{{INT_NUM_SITES}}{{SUFFIX_DASH}}/{{NUM_VEHICLES}}/opt_log",
            SEED=rob_seeds_strat,
            GROUP = rob_groups_strat,
        )
    output:
         OUTPUT_PREFIX + "/opt/techcase/robust/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/{BATTERY}/lowest_seed_opt_log_filename_tol{TOLERANCE}"
    shell:
        "python {input.scriptE} {input.optLogs} > {output}"




rule symlink_seed_robust_opt_log_file_name_tc:
    input:
         OUTPUT_PREFIX + "/opt/techcase/robust/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/{BATTERY}/{SEED_TYPE}_seed_opt_log_filename_tol{TOLERANCE}"
    output:
         OUTPUT_PREFIX + "/opt/techcase/robust/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/{BATTERY}/{SEED_TYPE}_opt_log_tol{TOLERANCE}"
    shell:
        "cp $(cat {input}) {output}"


