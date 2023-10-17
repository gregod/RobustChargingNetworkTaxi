import re

rob_seeds_strat=config["robust"]["seeds"]
rob_validation_seeds_strat=config["robust"]["validationSeeds"]
rob_groups_strat=config["robust"]["groups"]



rule run_robust_full_strat:
    group: "solving_bucket"
    threads: 1
    input:
        sites=OUTPUT_PREFIX + "/preprocessed/{INT_NUM_SITES}{SUFFIX_DASH}.sites.csv",
        vehicles=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{INT_NUM_SITES}}/{{NUM_VEHICLES}}/{{BATTERY}}.final.vehicles.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
        trips=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{INT_NUM_SITES}}/{{NUM_VEHICLES}}/{{BATTERY}}.final.trips.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        binary=OUTPUT_PREFIX + "/binaries/solution_approach_variable"

    output:
        stdout=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{INT_NUM_SITES}{SUFFIX_DASH}/{SITE_SIZE}/{NUM_VEHICLES}/full_opt_log_quorum:{QUORUM_ACCEPT}",
    shell:
        "timeout --foreground 10h {input.binary}  --vehicles  {input.vehicles} --trips  {input.trips}  --sites {input.sites} --battery {input.battery} --quorum_accept_percent={wildcards.QUORUM_ACCEPT} --activate_all > {output.stdout}"



rule generate_seed_median_robust_opt_log_file_name:
    conda:
         "environment.yaml"
    input:
        scriptE="preprocessing/find_median_opt_log.py",
        optLogs=expand(OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{{BATTERY}}/{{NUM_SITES}}/{{SITE_SIZE}}/{{NUM_VEHICLES}}/opt_log",SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
    output:
         OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{NUM_SITES}/{SITE_SIZE}/{NUM_VEHICLES}/median_seed_{is_leveled}opt_log_filename_tol{TOLERANCE}"
    shell:
        "python {input.scriptE} {input.optLogs} > {output}"


rule generate_seed_lowest_robust_opt_log_file_name:
    conda:
         "environment.yaml"
    input:
        scriptE="preprocessing/find_lowest_opt_log.py",
        optLogs=expand(OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{{BATTERY}}/{{NUM_SITES}}/{{SITE_SIZE}}/{{NUM_VEHICLES}}/{{is_leveled}}opt_log",SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
    output:
         OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{NUM_SITES}/{SITE_SIZE}/{NUM_VEHICLES}/lowest_seed_{is_leveled}opt_log_filename_tol{TOLERANCE}"
    shell:
        "python {input.scriptE} {input.optLogs} > {output}"


rule cp_seed_robust_opt_log:
    input:
         OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{NUM_SITES}/{SITE_SIZE}/{NUM_VEHICLES}/{SEED_TYPE}_seed_{is_leveled}opt_log_filename_tol{TOLERANCE}"
    output:
        OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{NUM_SITES}/{SITE_SIZE}/{NUM_VEHICLES}/{SEED_TYPE}_{is_leveled}opt_log_tol{TOLERANCE}"
    shell:
        "cp $(cat {input}) {output}"


rule run_robust_strategy:
    group: "solving_bucket"
    conda:
         "environment.yaml"
    threads: 1
    input:
        sites=OUTPUT_PREFIX + "/preprocessed/{INT_NUM_SITES}{SUFFIX_DASH}.sites.csv",
        seed_vehicle_opt_log=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{INT_NUM_SITES}{SUFFIX_DASH}/{SITE_SIZE}/{NUM_VEHICLES}/{SEED_TYPE}_seed_opt_log_filename_tol00",
        vehicles=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{INT_NUM_SITES}}/{{NUM_VEHICLES}}/{{BATTERY}}.final.vehicles.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
        trips=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{INT_NUM_SITES}}/{{NUM_VEHICLES}}/{{BATTERY}}.final.trips.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        opt_log_to_vehicles="preprocessing/opt_log_to_vehicles.py",
        opt_log_to_trips="preprocessing/opt_log_to_trips.py",
        opt_log_to_cuts="preprocessing/opt_log_to_cuts.py",
        binary=OUTPUT_PREFIX + "/binaries/solution_approach_variable"

    output:
        stdout=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{INT_NUM_SITES}{SUFFIX_DASH}/{SITE_SIZE}/{NUM_VEHICLES}/{SEED_TYPE}_opt_log_quorum:{QUORUM_ACCEPT}_activate:{MAX_ACTIVATE}_benevolent:{BENEVOLENT}_iis:{IIS}",
    shell:
        "timeout --foreground 10h {input.binary}  --site_size={wildcards.SITE_SIZE} --vehicles $(python {input.opt_log_to_vehicles} $(cat {input.seed_vehicle_opt_log})) {input.vehicles} --trips $(python {input.opt_log_to_trips} $(cat {input.seed_vehicle_opt_log})) {input.trips}  --sites {input.sites} --battery {input.battery} --cuts_input  $(python {input.opt_log_to_cuts} $(cat {input.seed_vehicle_opt_log}))   --max_activate_per_generation={wildcards.MAX_ACTIVATE} --quorum_accept_percent={wildcards.QUORUM_ACCEPT} --activate_iis={wildcards.IIS} --benevolent_accept_percent={wildcards.BENEVOLENT}  > {output.stdout}"



rule extract_site_csv_robust_strategy:
    group: "solving_bucket"
    conda:
        "environment.yaml"
    input:
         script="preprocessing/extract_site_results.py",
         sites=OUTPUT_PREFIX + "/preprocessed/{NUM_SITES}.sites.csv",
         optlog=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{NUM_SITES}/{SITE_SIZE}/{NUM_VEHICLES}/{PREFIX}_opt_log_{SUFFIX}"
    output:
          OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{NUM_SITES}/{SITE_SIZE}/{NUM_VEHICLES}/{PREFIX}_active_sites_{SUFFIX}",
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.optlog} {input.sites} > {output}"

rule check_cross_feasibility_of_robust_strat:
    group : "cross_checking"
    resources:
             runtime=1800
    input:
         active_sites=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{INT_NUM_SITES}{SUFFIX_DASH}/{SITE_SIZE}/{NUM_VEHICLES}/{PREFIX}_active_sites_{SUFFIX}",
         vehicles=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{INT_NUM_SITES}}/{{NUM_VEHICLES}}/{{BATTERY}}.final.vehicles.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
         trips=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{INT_NUM_SITES}}/{{NUM_VEHICLES}}/{{BATTERY}}.final.trips.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
         battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
         binary=OUTPUT_PREFIX + "/binaries/check_cross_feasibility"
    output:
          OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{INT_NUM_SITES}{SUFFIX_DASH}/{SITE_SIZE}/{NUM_VEHICLES}/{PREFIX}_cross_feasibility_{SUFFIX}"
    shell:
         "{input.binary}  --site_size={wildcards.SITE_SIZE} --percent_infeasible_allowed 0.00  --sites {input.active_sites} --battery {input.battery}  --trips {input.trips} --vehicles {input.vehicles} | grep -v '^Academic license' > {output}"

rule check_validation_cross_feasibility_of_robust_strat:
    group : "cross_checking"
    resources:
             runtime=1800
    input:
         active_sites=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{INT_NUM_SITES}{SUFFIX_DASH}/{SITE_SIZE}/{NUM_VEHICLES}/{PREFIX}_active_sites_{SUFFIX}",
         vehicles=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{INT_NUM_SITES}}/{{NUM_VEHICLES}}/{{BATTERY}}.final.vehicles.csv.gz", SEED=rob_validation_seeds_strat, TYPE_GROUP=rob_groups_strat),
         trips=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{INT_NUM_SITES}}/{{NUM_VEHICLES}}/{{BATTERY}}.final.trips.csv.gz", SEED=rob_validation_seeds_strat, TYPE_GROUP=rob_groups_strat),
         battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
         binary=OUTPUT_PREFIX + "/binaries/check_cross_feasibility"
    output:
          OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{INT_NUM_SITES}{SUFFIX_DASH}/{SITE_SIZE}/{NUM_VEHICLES}/{PREFIX}_validation_cross_feasibility_{SUFFIX}"
    shell:
         "{input.binary} --site_size={wildcards.SITE_SIZE} --percent_infeasible_allowed 0.00  --sites {input.active_sites} --battery {input.battery}  --trips {input.trips} --vehicles {input.vehicles} | grep -v '^Academic license' > {output}"


rule evaluate_cross_feasibility_of_robust_strat:
    group : "cross_checking"
    conda:
        "environment.yaml"
    resources:
             runtime=1800
    input:
        cross_file=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{NUM_SITES}/{SITE_SIZE}/{NUM_VEHICLES}/{PREFIX}_cross_feasibility_{SUFFIX}",
        script="preprocessing/evaluate_cross_result.py"
    output:
        cross_file=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{NUM_SITES}/{SITE_SIZE}/{NUM_VEHICLES}/{PREFIX}_percent_feasible_{SUFFIX}"
    shell:
        "python {input.script}  {input.cross_file} {wildcards.NUM_VEHICLES} > {output}"
