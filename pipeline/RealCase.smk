include: "common.smk"
RC_BASE_BATTERY = "battery_1"
DAY_RANGE=["{:02}".format(i) for i in range(1,31 +1) ]

rule calculate_battery:
    group: "preprocessing"
    conda:
        "environment.yaml"
    resources:
        runtime=3600
    input:
        battery="input_data/" + RC_BASE_BATTERY +".toml",
        script="preprocessing/create_rc_battery.py"
    output:
        battery=OUTPUT_PREFIX + "/preprocessed/realcase/raw_dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml"
    shell:
        "python {input.script} {input.battery} {wildcards.DBAT} {wildcards.DCHAR} {wildcards.DFINAL} > {output.battery}"


rule calc_battery:
    group: "preprocessing"
    conda:
        "environment.yaml"
    resources:
        runtime=3600
    input:
        battery=OUTPUT_PREFIX + "/preprocessed/realcase/raw_dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml",
        script="preprocessing/generate_charging_function.py"
    output:
        battery=OUTPUT_PREFIX + "/preprocessed/realcase/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml"
    shell:
        "python {input.script} {input.battery} > {output.battery}"

def get_fixed_trips_for_cap(wildcards):
    sdate = date(2015, 5, int(wildcards.DAY))  
    return f"{OUTPUT_PREFIX}/preprocessed/group_{sdate.weekday()}/taxi_trips_fixed.csv.gz"

rule cap_rc_trip_length:
    group: "preprocessing"
    conda:
        "environment.yaml"
    input:
        script="preprocessing/cap_vehicle_duration.py",
        vehicles=OUTPUT_PREFIX+"/preprocessed/vehicles/2015-05-{DAY}-00:00:00.csv.gz",
        trips=get_fixed_trips_for_cap
    output:
        vehicles=OUTPUT_PREFIX + "/preprocessed/realcase/vehicles.capped_{CAP}.{DAY,\d\d}.csv.gz",
    shell:
        "python {input.script} {input.vehicles} {input.trips} {wildcards.CAP} | gzip > {output.vehicles}"


def get_fixed_trips_for_processed(wildcards):
    sdate = date(2015, 5, int(wildcards.DAY))  
    return f"{OUTPUT_PREFIX}/preprocessed/group_{sdate.weekday()}/fall.processed.trips.csv.gz", # connect to main study processed trips


rule get_only_vehicle_trips:
    resources:
             runtime=1800
    group: "realcase"
    conda:
        "environment.yaml"
    input:
         script="preprocessing/keep_vehicle_trips.py",
         vehicles=OUTPUT_PREFIX + "/preprocessed/realcase/vehicles.capped_8.{DAY}.csv.gz",
         trips=get_fixed_trips_for_processed
    output:
          trips=OUTPUT_PREFIX+ "/preprocessed/realcase/trips.prefix.{DAY,\d\d}.csv.gz"
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.vehicles} {input.trips} {output.trips}"



rule process_trips_for_rc:
    resources:
             runtime=1800, mem_mb=1000
    group: "preprocessing"
    input:
         script="preprocessing/process_trips_orig.jl",
         taxi_trips=OUTPUT_PREFIX+ "/preprocessed/realcase/trips.prefix.{DAY}.csv.gz",
         taxi_sites=OUTPUT_PREFIX + "/preprocessed/nocost_60.sites.csv",
    output:
          taxi_trips=OUTPUT_PREFIX+ "/preprocessed/realcase/trips.all.{DAY,\d\d}.csv.gz"
    conda:
         "environment.yaml"
    shell:
         "julia --project=. {input.script} --sites {input.taxi_sites} --trips {input.taxi_trips} --output {output}"


# filter those that are in
rule remove_rc_notfully_feasible:
    group: "preprocessing"
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/realcase/vehicles.capped_8.{DAY}.csv.gz",
        trips=OUTPUT_PREFIX+ "/preprocessed/realcase/trips.all.{DAY}.csv.gz",
        sites="input_data/taxi_sites.csv",
        battery=OUTPUT_PREFIX +"/preprocessed/realcase/dbat:2.00_dcha:2.00_dfin:1.00.toml",
        binary=OUTPUT_PREFIX + "/binaries/remove_infeasible"
    output:
        vehicles=OUTPUT_PREFIX+ "/preprocessed/realcase/vehicles.feasible.{DAY}.csv.gz"
    shell:
        "{input.binary} --vehicles {input.vehicles} --trips {input.trips} --sites {input.sites} --battery {input.battery} | gzip  > {output.vehicles}"

# Do not do any prefiltering !!!!
#rule passthrough_vehicles_rc:
#    group: "preprocessing"
#    input:
#        vehicles=OUTPUT_PREFIX + "/preprocessed/realcase/vehicles.capped_8.{DAY}.csv.gz",
#    output:
#        vehicles=OUTPUT_PREFIX+ "/preprocessed/realcase/vehicles.feasible.{DAY,\d\d}.csv.gz"
#    shell:
#        "cp {input.vehicles} {output.vehicles}"

rule get_base:
    group: "preprocessing"
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/realcase/vehicles.feasible.{DAY}.csv.gz",
        trips=OUTPUT_PREFIX+ "/preprocessed/realcase/trips.all.{DAY}.csv.gz",
        sites=OUTPUT_PREFIX+"/preprocessed/nocost_60.sites.csv",
        battery=OUTPUT_PREFIX +"/preprocessed/realcase/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml",
        binary=OUTPUT_PREFIX + "/binaries/remove_infeasible"
    output:
        vehicles=OUTPUT_PREFIX+ "/preprocessed/realcase/vehicles.base.dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.{DAY}.csv.gz"
    shell:
        "{input.binary} --vehicles {input.vehicles} --trips {input.trips} --sites {input.sites} --battery {input.battery} | gzip  > {output.vehicles}"



rule run_opt_on_rc:
    group: "solving_bucket"
    threads: 1
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/realcase/vehicles.base.dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.{DAY}.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/realcase/trips.all.{DAY}.csv.gz",
        sites=OUTPUT_PREFIX + "/preprocessed/60.sites.csv",
        battery=OUTPUT_PREFIX +"/preprocessed/realcase/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml",
        binary=OUTPUT_PREFIX + "/binaries/solution_approach"
    output:
        stdout=OUTPUT_PREFIX + "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/opt_log",
        charge_process=OUTPUT_PREFIX + "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/opt_chargeprocess",
        cuts=OUTPUT_PREFIX + "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/opt_cuts"
    log:
        trace=OUTPUT_PREFIX + "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/opt_log_trace.bin"
    shell:
        "{input.binary}  --vehicles {input.vehicles} --trips {input.trips} --cuts_output {output.cuts}  --sites {input.sites} --battery {input.battery} --sites_min=1 --charge_processes_file {output.charge_process} --hawktracer_output {log.trace} > {output.stdout}"

rule extract_site_csv_realcase:
    group: "solving_bucket"
    input:
         script="preprocessing/extract_site_results.py",
         sites=OUTPUT_PREFIX + "/preprocessed/60.sites.csv",
         optlog=OUTPUT_PREFIX + "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/opt_log",
    output:
          OUTPUT_PREFIX + "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/active_sites",
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.optlog} {input.sites} > {output}"



rule check_cross_feasibility_of_rc:
    group : "cross_checking"
    resources:
             runtime=1800
    input:
         active_sites=OUTPUT_PREFIX + "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/active_sites",
         vehicles=expand(OUTPUT_PREFIX + "/preprocessed/realcase/vehicles.base.dbat:{{DBAT}}_dcha:{{DCHAR}}_dfin:{{DFINAL}}.{DAY}.csv.gz",
            DAY=DAY_RANGE
         ),
         trips=expand(OUTPUT_PREFIX  + "/preprocessed/realcase/trips.all.{DAY}.csv.gz",
            DAY=DAY_RANGE
         ),
         battery=OUTPUT_PREFIX +"/preprocessed/realcase/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml",
         binary=OUTPUT_PREFIX + "/binaries/check_cross_feasibility"
    output:
          OUTPUT_PREFIX + "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/cross_feasibility"
    shell:
         "{input.binary} --percent_infeasible_allowed 0.00  --sites {input.active_sites} --battery {input.battery}  --trips {input.trips} --vehicles {input.vehicles} | grep -v '^Academic license' > {output}"


rule evaluate_cross_feasibility_of_rc:
    group : "cross_checking"
    conda:
        "environment.yaml"
    resources:
             runtime=1800
    input:
        cross_file=OUTPUT_PREFIX + "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/cross_feasibility",
        script="preprocessing/evaluate_cross_result_rc.py"
    output:
        cross_file=OUTPUT_PREFIX + "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/percent_feasible"
    shell:
        "python {input.script}  {input.cross_file} > {output}"







rule run_full_robust:
    group: "solving_bucket"
    threads: 1
    input:
        sites=OUTPUT_PREFIX + "/preprocessed/60.sites.csv",
        vehicles=expand(OUTPUT_PREFIX + "/preprocessed/realcase/vehicles.base.dbat:{{DBAT}}_dcha:{{DCHAR}}_dfin:{{DFINAL}}.{DAY}.csv.gz",
            DAY=DAY_RANGE
        ),
        trips=expand(OUTPUT_PREFIX  + "/preprocessed/realcase/trips.all.{DAY}.csv.gz",
            DAY=DAY_RANGE
        ),
        battery=OUTPUT_PREFIX +"/preprocessed/realcase/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml",
        binary=OUTPUT_PREFIX + "/binaries/solution_approach_robust"
    output:
        stdout=OUTPUT_PREFIX + "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/full_opt_log_quorum:{QUORUM_ACCEPT}",
    shell:
        "{input.binary}  --vehicles  {input.vehicles} --trips  {input.trips}  --sites {input.sites} --battery {input.battery} --quorum_accept_percent={wildcards.QUORUM_ACCEPT} --activate_all > {output.stdout}"


rule generate_median_case_rc:
    conda:
        "environment.yaml"
    input:
        script="preprocessing/merge_cases.py",
        vehicles=expand(OUTPUT_PREFIX+ "/preprocessed/realcase/vehicles.feasible.{DAY}.csv.gz", DAY=DAY_RANGE),
        trips=expand(OUTPUT_PREFIX  + "/preprocessed/realcase/trips.all.{DAY}.csv.gz",DAY=DAY_RANGE),
    output:
        vehicles=OUTPUT_PREFIX+"/preprocessed/realcase/vehicles.merged.csv.gz",
        trips=OUTPUT_PREFIX+"/preprocessed/realcase/trips.merged.csv.gz"
    shell:
        "python {input.script} {input.vehicles} {input.trips} {output.vehicles} {output.trips}"

rule determine_time_distributions_for_median_case:
    group: "preprocessing"
    conda:
        "environment.yaml"
    input:
        script="preprocessing/determine_distribution.py",
        vehicles=OUTPUT_PREFIX+"/preprocessed/realcase/vehicles.merged.csv.gz",
        trips=OUTPUT_PREFIX+"/preprocessed/realcase/trips.merged.csv.gz"
    output:
        dist=OUTPUT_PREFIX+"/preprocessed/realcase/merged.distribution.txt"
    shell:
        "python {input.script} {input.vehicles} {input.trips} > {output.dist}"


rule generate_sample_for_median_rc_case:
    group: "preprocessing"
    conda:
        "environment.yaml"
    input:
        script="preprocessing/generate_sample.py",
        vehicles=OUTPUT_PREFIX+"/preprocessed/realcase/vehicles.merged.csv.gz",
        trips=OUTPUT_PREFIX+"/preprocessed/realcase/trips.merged.csv.gz",
        dist=OUTPUT_PREFIX+"/preprocessed/realcase/merged.distribution.txt"
    output:
        vehicles=OUTPUT_PREFIX+"/preprocessed/realcase/vehicles.feasible.median{SEED}.csv.gz",
        trips=OUTPUT_PREFIX+"/preprocessed/realcase/trips.all.median{SEED}.csv.gz"
    shell:
        "python {input.script} {input.vehicles} {input.trips} {input.dist} 260 {wildcards.SEED} {output.vehicles} {output.trips}"




rule generate_seed_median_robust_opt_log_file_name_rc:
    conda:
        "environment.yaml"
    input:
        scriptE="preprocessing/find_median_opt_log.py",
        optLogs=expand(OUTPUT_PREFIX + "/opt/realcase/{DAY}/dbat:{{DBAT}}_dcha:{{DCHAR}}_dfin:{{DFINAL}}/opt_log",DAY=DAY_RANGE),
    output:
         OUTPUT_PREFIX + "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/median_seed_opt_log_filename_tol00"
    shell:
        "python {input.scriptE} {input.optLogs} > {output}"


rule generate_seed_lowest_robust_opt_log_file_name_rc:
    conda:
        "environment.yaml"
    input:
        scriptE="preprocessing/find_lowest_opt_log.py",
        optLogs=expand(OUTPUT_PREFIX + "/opt/realcase/{DAY}/dbat:{{DBAT}}_dcha:{{DCHAR}}_dfin:{{DFINAL}}/opt_log",DAY=DAY_RANGE),
    output:
         OUTPUT_PREFIX + "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/lowest_seed_opt_log_filename_tol00"
    shell:
        "python {input.scriptE} {input.optLogs} > {output}"


rule symlink_seed_robust_opt_log_file_name_rc:
    input:
         OUTPUT_PREFIX + "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{SEED_TYPE}_seed_opt_log_filename_tol00"
    output:
         OUTPUT_PREFIX + "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{SEED_TYPE}_opt_log_tol00"
    shell:
        "cp $(cat {input}) {output}"


rule run_robust2_rc:
    group: "solving_bucket"
    conda:
        "environment.yaml"
    threads: 1
    input:
        sites=OUTPUT_PREFIX + "/preprocessed/60.sites.csv",
        seed_vehicle_opt_log=OUTPUT_PREFIX + "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{SEED_TYPE}_seed_opt_log_filename_tol00",
        vehicles=expand(OUTPUT_PREFIX + "/preprocessed/realcase/vehicles.base.dbat:{{DBAT}}_dcha:{{DCHAR}}_dfin:{{DFINAL}}.{DAY}.csv.gz",
            DAY=DAY_RANGE
        ),
        trips=expand(OUTPUT_PREFIX  + "/preprocessed/realcase/trips.all.{DAY}.csv.gz",
            DAY=DAY_RANGE
        ),
        battery=OUTPUT_PREFIX +"/preprocessed/realcase/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml",
        opt_log_to_vehicles="preprocessing/opt_log_to_vehicles_rc.py",
        opt_log_to_trips="preprocessing/opt_log_to_trips_rc.py",
        opt_log_to_cuts="preprocessing/opt_log_to_cuts.py",
        binary=OUTPUT_PREFIX + "/binaries/robust2"

    output:
        stdout=OUTPUT_PREFIX + "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{SEED_TYPE}_opt_log_quorum:{QUORUM_ACCEPT}_activate:{MAX_ACTIVATE}_benevolent:{BENEVOLENT}_iis:{IIS}"
    shell:
        "{input.binary}  --vehicles $(python {input.opt_log_to_vehicles} $(cat {input.seed_vehicle_opt_log})) {input.vehicles} --trips $(python {input.opt_log_to_trips} $(cat {input.seed_vehicle_opt_log})) {input.trips}  --sites {input.sites} --battery {input.battery} --cuts_input  $(python {input.opt_log_to_cuts} $(cat {input.seed_vehicle_opt_log}))   --max_activate_per_generation={wildcards.MAX_ACTIVATE} --quorum_accept_percent={wildcards.QUORUM_ACCEPT} --activate_iis={wildcards.IIS} --benevolent_accept_percent={wildcards.BENEVOLENT}  > {output.stdout}"


rule extract_site_csv_robust_rc:
    group: "solving_bucket"
    conda:
        "environment.yaml"
    input:
         script="preprocessing/extract_site_results.py",
         sites=OUTPUT_PREFIX + "/preprocessed/60.sites.csv",
         optlog=OUTPUT_PREFIX + "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{PREFIX}_opt_log_{SUFFIX}"
    output:
          OUTPUT_PREFIX + "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{PREFIX}_active_sites_{SUFFIX}"
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.optlog} {input.sites} > {output}"

rule check_cross_feasibility_of_robust_rc:
    group : "cross_checking"
    resources:
             runtime=1800
    input:
         active_sites=OUTPUT_PREFIX + "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{PREFIX}_active_sites_{SUFFIX}",
         vehicles=expand(OUTPUT_PREFIX + "/preprocessed/realcase/vehicles.base.dbat:{{DBAT}}_dcha:{{DCHAR}}_dfin:{{DFINAL}}.{DAY}.csv.gz",
            DAY=DAY_RANGE
         ),
         trips=expand(OUTPUT_PREFIX  + "/preprocessed/realcase/trips.all.{DAY}.csv.gz",
            DAY=DAY_RANGE
         ),
         battery=OUTPUT_PREFIX +"/preprocessed/realcase/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.toml",
         binary=OUTPUT_PREFIX + "/binaries/check_cross_feasibility"
    output:
          OUTPUT_PREFIX + "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{PREFIX}_cross_feasibility_{SUFFIX}"
    shell:
         "{input.binary} --percent_infeasible_allowed 0.00  --sites {input.active_sites} --battery {input.battery}  --trips {input.trips} --vehicles {input.vehicles} | grep -v '^Academic license' > {output}"


rule evaluate_cross_feasibility_of_robust_rc:
    group : "cross_checking"
    conda:
        "environment.yaml"
    resources:
             runtime=1800
    input:
        cross_file=OUTPUT_PREFIX + "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{PREFIX}_cross_feasibility_{SUFFIX}",
        script="preprocessing/evaluate_cross_result_rc.py"
    output:
        cross_file=OUTPUT_PREFIX + "/opt/realcase/robust/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/{PREFIX}_percent_feasible_{SUFFIX}"
    shell: # TODO: FIX the 500 percent feasible metric to be relative to num vehicles of day!
        "python {input.script}  {input.cross_file} > {output}"



SEN_RANGE=["0.85","0.90","0.95","1.00","1.05","1.10","1.15"]

rule generate_base:
    output: OUTPUT_PREFIX + "/preprocessed/realcase/flag1"
    input:
        expand(OUTPUT_PREFIX+  "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/opt_log",
            DAY=DAY_RANGE,DBAT=SEN_RANGE,DCHAR="1.00",DFINAL="1.00"
        ),
        expand(OUTPUT_PREFIX+  "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/opt_log",
            DAY=DAY_RANGE,DBAT="1.00",DCHAR=SEN_RANGE,DFINAL="1.00"
        ),
        expand(OUTPUT_PREFIX+  "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/opt_log",
            DAY=DAY_RANGE,DBAT="1.00",DCHAR="1.00",DFINAL=SEN_RANGE
        )
    shell: 
        "echo 1 > {output}"

