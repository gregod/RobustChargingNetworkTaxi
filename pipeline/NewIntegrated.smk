
rob_seeds_strat=config["robust"]["seeds"]
rob_groups_strat=config["robust"]["groups"]

rule gen_sites_old_strategy:
    group: "preprocessing"
    resources:
             runtime=3600
    input:
         OUTPUT_PREFIX + "/preprocessed/nocost_{SITE_COUNT}.sites.csv"
    output:
          OUTPUT_PREFIX + "/preprocessed/nocost_strat:old_{SITE_COUNT}.sites.csv"
    shell:
         "cat {input} > {output}"

rule gen_sites_kde_strategy:
    group: "preprocessing"
    conda:
        "environment.yaml"
    resources:
             runtime=3600
    input:
         script="preprocessing/kde_sampler.py",
         taxi_sites="input_data/taxi_sites.csv",
         trips="input_data/trips.csv.gz"
    output:
          OUTPUT_PREFIX + "/preprocessed/nocost_strat:kde_{SITE_COUNT}.sites.csv"
    shell:
         "python {input.script} {input.taxi_sites} {input.trips} {wildcards.SITE_COUNT}  > {output}"



rule gen_sites_sample_circle_strategy:
    group: "preprocessing"
    resources:
             runtime=3600
    input:
         script="preprocessing/site_sampler_circles.jl",
         taxi_sites="input_data/taxi_sites.csv"
    output:
          OUTPUT_PREFIX + "/preprocessed/nocost_strat:circle_{CIRCLE_SIZE}_{INNER_COUNT}_{OUTER_COUNT}.sites.csv"
    shell:
         "julia --project=. --optimize=0 {input.script} --sites {input.taxi_sites} --inner-circle-size {wildcards.CIRCLE_SIZE} --inner-count {wildcards.INNER_COUNT} --outer-count {wildcards.OUTER_COUNT} > {output}"



rule gen_sites_sample_three_circle_strategy:
    group: "preprocessing"
    resources:
             runtime=3600
    input:
         script="preprocessing/site_sampler_three_circles.jl",
         taxi_sites="input_data/taxi_sites.csv"
    output:
          OUTPUT_PREFIX + "/preprocessed/nocost_strat:threecircle_{CIRCLE_SIZE_INNER}-{CIRCLE_SIZE_MIDDLE}_{INNER_COUNT}-{MIDDLE_COUNT}-{OUTER_COUNT}.sites.csv"
    shell:
         "julia --project=. --optimize=0 {input.script} --sites {input.taxi_sites} --inner-circle-size {wildcards.CIRCLE_SIZE_INNER} --middle-circle-size {wildcards.CIRCLE_SIZE_MIDDLE} --inner-count {wildcards.INNER_COUNT} --middle-count {wildcards.MIDDLE_COUNT} --outer-count {wildcards.OUTER_COUNT} > {output}"


rule add_costs_strat:
    group: "preprocessing"
    conda:
        "environment.yaml"
    resources:
             runtime=3600
    input:
         script="preprocessing/set_site_costs.py",
         taxi_sites=OUTPUT_PREFIX + "/preprocessed/nocost_strat:{SITE_STRATEGY}.sites.csv"
    output:
        taxi_sites=OUTPUT_PREFIX + "/preprocessed/strat:{SITE_STRATEGY}.sites.csv"
    shell:
         "python {input.script} {input.taxi_sites} > {output}"


rule generate_sample_for_group_all_strats:
    group: "preprocessing"
    conda:
        "environment.yaml"
    input:
        script="preprocessing/generate_sample.py",
        vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/battery_1.feasible.vehicles.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/fall.processed.trips.csv.gz",
        dist=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/distribution.txt",
    output:
        vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_VEHICLES}/sampled.vehicles.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_VEHICLES}/sampled.trips.csv.gz"
    shell:
        "python {input.script} {input.vehicles} {input.trips} {input.dist} {wildcards.NUM_VEHICLES} {wildcards.SEED} {output.vehicles} {output.trips}"

rule process_capped_trips:
    resources:
             runtime=1800, mem_mb=10000
    group: "preprocessing"
    input:
         script="preprocessing/process_trips_repl.jl",
         taxi_trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/taxi_trips_fixed.csv.gz",
         taxi_sites="input_data/taxi_sites.csv",
         distance_cache=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/distance_cache.jls"
    output:
          OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/fall.processed.trips.csv.gz"
    conda:
         "environment.yaml"
    shell:
         "julia --project=. {input.script} --sites {input.taxi_sites} --trips {input.taxi_trips} --distance-cache {input.distance_cache} --output {output}"

# take the sites and trips and produce file that contains the trips with reachable sites.
rule process_trips_for_strat:
    resources:
             runtime=1800, mem_mb=10000
    group: "preprocessing"
    input:
         script="preprocessing/process_trips_orig.jl",
         taxi_trips=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{NUM_VEHICLES}}/sampled.trips.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
         taxi_sites=OUTPUT_PREFIX + "/preprocessed/nocost_strat:{SITE_STRATEGY}.sites.csv",
    output:
          expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{NUM_VEHICLES}}/{{SITE_STRATEGY}}/fixed.trips.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat)
    conda:
         "environment.yaml"
    shell:
         "julia --project=. {input.script} --sites {input.taxi_sites} --trips {input.taxi_trips} --output {output}"

rule remove_notfully_feasible:
    group: "preprocessing"
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/capped_8.vehicles.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/fall.processed.trips.csv.gz",
        sites="input_data/taxi_sites.csv",
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        binary=OUTPUT_PREFIX + "/binaries/remove_infeasible"
    output:
        vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{BATTERY}.feasible.vehicles.csv.gz",
    shell:
        "{input.binary} --vehicles {input.vehicles} --trips {input.trips} --sites {input.sites} --battery {input.battery} | gzip  > {output.vehicles}"


rule ensure_feasible_for_site_strat:
    group: "preprocessing"
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_VEHICLES}/sampled.vehicles.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_VEHICLES}/{SITE_STRATEGY}/fixed.trips.csv.gz",
        sites=OUTPUT_PREFIX + "/preprocessed/nocost_strat:{SITE_STRATEGY}.sites.csv",
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        binary=OUTPUT_PREFIX + "/binaries/remove_infeasible"
    output:
        vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_VEHICLES}/{SITE_STRATEGY}/{BATTERY}.feasible.vehicles.csv.gz"
    shell:
        "{input.binary} --vehicles {input.vehicles} --trips {input.trips} --sites {input.sites} --battery {input.battery} | gzip  > {output.vehicles}"




rule run_opt_on_group_strat:
    group: "solving_bucket"
    threads: 1
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_VEHICLES}/{SITE_STRATEGY}/{BATTERY}.feasible.vehicles.csv.gz",
        sites=OUTPUT_PREFIX + "/preprocessed/strat:{SITE_STRATEGY}.sites.csv",
        trips=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_VEHICLES}/{SITE_STRATEGY}/fixed.trips.csv.gz",
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        binary=OUTPUT_PREFIX + "/binaries/benders"
    output:
        stdout=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{SITE_STRATEGY}/{NUM_VEHICLES}/opt_log",
        charge_process=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{SITE_STRATEGY}/{NUM_VEHICLES}/opt_chargeprocess",
        cuts=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{SITE_STRATEGY}/{NUM_VEHICLES}/opt_cuts"
    log:
        trace=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{SITE_STRATEGY}/{NUM_VEHICLES}/opt_log_trace.bin"
    shell:
        "{input.binary}  --vehicles {input.vehicles} --trips {input.trips} --cuts_output {output.cuts}  --sites {input.sites} --battery {input.battery} --sites_min=1 --percent_infeasible_allowed 0.{wildcards.TOLERANCE} --charge_processes_file {output.charge_process} --hawktracer_output {log.trace} > {output.stdout}"

rule extract_site_csv_strat:
    group: "solving_bucket"
    conda:
        "environment.yaml"
    input:
         script="preprocessing/extract_site_results.py",
         sites=OUTPUT_PREFIX + "/preprocessed/strat:{SITE_STRATEGY}.sites.csv",
         optlog=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{SITE_STRATEGY}/{NUM_VEHICLES}/opt_log",
    output:
          OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{SITE_STRATEGY}/{NUM_VEHICLES}/active_sites.csv",
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.optlog} {input.sites} > {output}"



rule run_robust2_full_strat:
    group: "solving_bucket"
    conda:
        "environment.yaml"
    threads: 1
    input:
        sites=OUTPUT_PREFIX + "/preprocessed/strat:{SITE_STRATEGY}.sites.csv",
        vehicles=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{NUM_VEHICLES}}/{{SITE_STRATEGY}}/{{BATTERY}}.feasible.vehicles.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
        trips=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{NUM_VEHICLES}}/{{SITE_STRATEGY}}/fixed.trips.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        binary=OUTPUT_PREFIX + "/binaries/robust2"

    output:
        stdout=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/full_opt_log_quorum:{QUORUM_ACCEPT}",
    shell:
        "{input.binary}  --vehicles  {input.vehicles} --trips  {input.trips}  --sites {input.sites} --battery {input.battery} --quorum_accept_percent={wildcards.QUORUM_ACCEPT} --activate_all > {output.stdout}"




rule generate_seed_median_robust_opt_log_file_name_srat:
    conda:
        "environment.yaml"
    input:
        scriptE="preprocessing/find_median_opt_log.py",
        optLogs=expand(OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{{BATTERY}}/tol{{TOLERANCE}}/{{SITE_STRATEGY}}/{{NUM_VEHICLES}}/opt_log",SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
    output:
         OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/median_seed_{is_leveled}opt_log_filename_tol{TOLERANCE}"
    shell:
        "python {input.scriptE} {input.optLogs} > {output}"


rule generate_seed_lowest_robust_opt_log_file_name_srat:
    conda:
        "environment.yaml"
    input:
        scriptE="preprocessing/find_lowest_opt_log.py",
        optLogs=expand(OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{{BATTERY}}/tol{{TOLERANCE}}/{{SITE_STRATEGY}}/{{NUM_VEHICLES}}/{{is_leveled}}opt_log",SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
    output:
         OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/lowest_seed_{is_leveled}opt_log_filename_tol{TOLERANCE}"
    shell:
        "python {input.scriptE} {input.optLogs} > {output}"


rule symlink_seed_robust_opt_log_file_name_srat:
    input:
         OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/{SEED_TYPE}_seed_{is_leveled}opt_log_filename_tol{TOLERANCE}"
    output:
        OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/{SEED_TYPE}_{is_leveled}opt_log_tol{TOLERANCE}"
    shell:
        "cp $(cat {input}) {output}"


rule run_robust2_srat:
    group: "solving_bucket"
    conda:
        "environment.yaml"
    threads: 1
    input:
        sites=OUTPUT_PREFIX + "/preprocessed/strat:{SITE_STRATEGY}.sites.csv",
        seed_vehicle_opt_log=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/{SEED_TYPE}_seed_opt_log_filename_tol00",
        vehicles=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{NUM_VEHICLES}}/{{SITE_STRATEGY}}/{{BATTERY}}.feasible.vehicles.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
        trips=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{NUM_VEHICLES}}/{{SITE_STRATEGY}}/fixed.trips.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        opt_log_to_vehicles="preprocessing/opt_log_to_vehicles_strat.py",
        opt_log_to_trips="preprocessing/opt_log_to_trips_strat.py",
        opt_log_to_cuts="preprocessing/opt_log_to_cuts.py",
        binary=OUTPUT_PREFIX + "/binaries/robust2"

    output:
        stdout=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/{SEED_TYPE}_opt_log_quorum:{QUORUM_ACCEPT}_activate:{MAX_ACTIVATE}_benevolent:{BENEVOLENT}_iis:{IIS}",
    shell:
        "{input.binary}  --vehicles $(python {input.opt_log_to_vehicles} $(cat {input.seed_vehicle_opt_log})) {input.vehicles} --trips $(python {input.opt_log_to_trips} $(cat {input.seed_vehicle_opt_log})) {input.trips}  --sites {input.sites} --battery {input.battery} --cuts_input  $(python {input.opt_log_to_cuts} $(cat {input.seed_vehicle_opt_log}))   --max_activate_per_generation={wildcards.MAX_ACTIVATE} --quorum_accept_percent={wildcards.QUORUM_ACCEPT} --activate_iis={wildcards.IIS} --benevolent_accept_percent={wildcards.BENEVOLENT}  > {output.stdout}"


rule run_robust2_srat_withvehicle_count:
    group: "solving_bucket"
    conda:
        "environment.yaml"
    threads: 1
    input:
        sites=OUTPUT_PREFIX + "/preprocessed/strat:{SITE_STRATEGY}.sites.csv",
        seed_vehicle_opt_log=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/{SEED_TYPE}_seed_opt_log_filename_tol00",
        vehicles=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{NUM_VEHICLES}}/{{SITE_STRATEGY}}/{{BATTERY}}.feasible.vehicles.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
        trips=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{NUM_VEHICLES}}/{{SITE_STRATEGY}}/fixed.trips.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        opt_log_to_vehicles="preprocessing/opt_log_to_vehicles_strat.py",
        opt_log_to_trips="preprocessing/opt_log_to_trips_strat.py",
        opt_log_to_cuts="preprocessing/opt_log_to_cuts.py",
        binary=OUTPUT_PREFIX + "/binaries/robust2"

    output:
        stdout=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/{SEED_TYPE}_opt_log_quorum:{QUORUM_ACCEPT}_activate:{MAX_ACTIVATE}_benevolent:{BENEVOLENT}_on:vehicles_iis:{IIS}",
    shell:
        "{input.binary} --total_num_vehicles {wildcards.NUM_VEHICLES}  --vehicles $(python {input.opt_log_to_vehicles} $(cat {input.seed_vehicle_opt_log})) {input.vehicles} --trips $(python {input.opt_log_to_trips} $(cat {input.seed_vehicle_opt_log})) {input.trips}  --sites {input.sites} --battery {input.battery} --cuts_input  $(python {input.opt_log_to_cuts} $(cat {input.seed_vehicle_opt_log}))   --max_activate_per_generation={wildcards.MAX_ACTIVATE} --quorum_accept_percent={wildcards.QUORUM_ACCEPT} --activate_iis={wildcards.IIS} --benevolent_accept_percent={wildcards.BENEVOLENT}  > {output.stdout}"




rule extract_site_csv_robust_strat:
    group: "solving_bucket"
    conda:
        "environment.yaml"
    input:
         script="preprocessing/extract_site_results.py",
         sites=OUTPUT_PREFIX + "/preprocessed/strat:{SITE_STRATEGY}.sites.csv",
         optlog=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/{PREFIX}_opt_log_{SUFFIX}"
    output:
          OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/{PREFIX}_active_sites_{SUFFIX}.csv",
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.optlog} {input.sites} > {output}"

rule check_cross_feasibility_of_robust_strat:
    group : "cross_checking"
    resources:
             runtime=1800
    input:
         active_sites=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/{PREFIX}_active_sites_{SUFFIX}.csv",
         vehicles=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{NUM_VEHICLES}}/{{SITE_STRATEGY}}/{{BATTERY}}.feasible.vehicles.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
         trips=expand(OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{{NUM_VEHICLES}}/{{SITE_STRATEGY}}/fixed.trips.csv.gz", SEED=rob_seeds_strat, TYPE_GROUP=rob_groups_strat),
         battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
         binary=OUTPUT_PREFIX + "/binaries/check_cross_feasibility"
    output:
          OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/{PREFIX}_cross_feasibility_{SUFFIX}"
    shell:
         "{input.binary} --percent_infeasible_allowed 0.00  --sites {input.active_sites} --battery {input.battery}  --trips {input.trips} --vehicles {input.vehicles} | grep -v '^Academic license' > {output}"


rule evaluate_cross_feasibility_of_robust_strat:
    group : "cross_checking"
    conda:
        "environment.yaml"
    resources:
             runtime=1800
    input:
        cross_file=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/{PREFIX}_cross_feasibility_{SUFFIX}",
        script="preprocessing/evaluate_cross_result.py"
    output:
        cross_file=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{SITE_STRATEGY}/{NUM_VEHICLES}/{PREFIX}_percent_feasible_{SUFFIX}"
    shell:
        "python {input.script}  {input.cross_file} {wildcards.NUM_VEHICLES} > {output}"
