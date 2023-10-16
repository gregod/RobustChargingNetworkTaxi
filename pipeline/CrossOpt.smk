
rule check_cross_feasibilty:
    group : "cross_checking"
    resources:
             runtime=1800
    input:
         active_sites=OUTPUT_PREFIX + "/opt/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}active_sites.csv",
         trips=generate_other_trips_input,
         vehicles=generate_other_vehicles_input,
         battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
         binary=OUTPUT_PREFIX + "/binaries/check_cross_feasibility"
    output:
          OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}cross_feasibility"
    shell:
         "{input.binary} --percent_infeasible_allowed 0.{wildcards.TOLERANCE}  --sites {input.active_sites} --battery {input.battery}  --trips {input.trips} --vehicles {input.vehicles} | grep -v '^Academic license' > {output}"

rule extract_cross_infeasible_into_single_case:
    group : "cross_checking"
    input:
         script="preprocessing/extract_infeasible_vehicles.py",
         cross_feasiblity=OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}cross_feasibility",
         orig_vehicles=OUTPUT_PREFIX + "/preprocessed/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/final.vehicles.csv",
         orig_trips=OUTPUT_PREFIX + "/preprocessed/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/final.trips.csv",
         _trips=generate_other_trips_input,
         _vehicles=generate_other_vehicles_input,
    conda:
         "environment.yaml"
    output:
          cross_vehicles=OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}cross.vehicles.csv",
          cross_trips=OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}cross.trips.csv"
    shell:
         "python {input.script}  {input.cross_feasiblity} {input.orig_vehicles} {input.orig_trips} {output.cross_trips} > {output.cross_vehicles}"

rule run_column_generation_on_cross_site_bucket:
    group: "cross_checking_solve"
    priority: 1
    resources:
             runtime=lambda wildcards, attempt: 3 * (60 * 60) if int(re.sub("[^0-9]+", " ", wildcards.NUM_SITES)) == 30 else 40 * (60*60), mem_mb=8129
    threads: 1
    input:
         vehicles=OUTPUT_PREFIX+ "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/cross.vehicles.csv",
         sites=OUTPUT_PREFIX + "/preprocessed/{NUM_SITES}.sites.csv",
         trips=OUTPUT_PREFIX +"/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/cross.trips.csv",
         battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
         cuts= OUTPUT_PREFIX +"/opt/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/opt_cuts",
         binary=OUTPUT_PREFIX + "/binaries/benders"
    output:
          stdout=OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/cross_opt_log",
          charge_process=OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/cross_opt_chargeprocess",
          cuts=OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/cross_opt_cuts"
    log:
          trace=OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/cross_opt_log_trace.bin",
    shell:
         "{input.binary} --percent_infeasible_allowed 0.{wildcards.TOLERANCE}  --vehicles {input.vehicles} --trips {input.trips} --cuts_input {input.cuts}  --cuts_output {output.cuts}   --sites {input.sites} --battery {input.battery} --sites_min=1 --charge_processes_file {output.charge_process}  --hawktracer_output {log.trace} > {output.stdout}"


rule run_var_column_generation_on_cross_site_bucket:
    group: "cross_checking_solve"

    resources:
             runtime=lambda wildcards, attempt: 3 * (60 * 60) if int(re.sub("[^0-9]+", " ", wildcards.NUM_SITES)) == 30 else 40 * (60*60), mem_mb=8129
    threads: 1
    input:
         vehicles=OUTPUT_PREFIX+ "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/var_cross.vehicles.csv",
         sites=OUTPUT_PREFIX + "/preprocessed/{NUM_SITES}.sites.csv",
         battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
         cuts= OUTPUT_PREFIX +"/opt/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/var_opt_cuts",
         trips=OUTPUT_PREFIX +"/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/var_cross.trips.csv",
         binary=OUTPUT_PREFIX + "/binaries/bendersVar"
    output:
          stdout=OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/var_cross_opt_log",
          charge_process=OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/var_cross_opt_chargeprocess",
          cuts=OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/var_cross_opt_cuts"
    log:
          trace=OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/var_cross_opt_log_trace.bin",
    shell:
         "{input.binary}   --percent_infeasible_allowed 0.{wildcards.TOLERANCE} --cuts_output {output.cuts} --cuts_input {input.cuts}   --vehicles {input.vehicles} --trips {input.trips}  --sites {input.sites} --battery {input.battery} --sites_min=1 --charge_processes_file {output.charge_process} --hawktracer_output {log.trace} > {output.stdout}"



rule extract_site_cross_csv:
    group: "cross_checking"
    resources:
             runtime=1800, mem_mb=1024
    input:
         script="preprocessing/extract_site_results.py",
         sites=OUTPUT_PREFIX + "/preprocessed/{NUM_SITES}.sites.csv",
         optlog=OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}cross_opt_log",
    output:
          OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}cross_active_sites.csv"
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.optlog} {input.sites} > {output}"


rule check_cross_feasibilty_of_cross:
    group: "cross_checking"
    input:
         active_sites=OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}cross_active_sites.csv",
         trips=generate_other_trips_input,
         vehicles=generate_other_vehicles_input,
         binary=OUTPUT_PREFIX + "/binaries/check_cross_feasibility"
    output:
          OUTPUT_PREFIX + "/cross/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}cross_cross_feasibility"
    shell:
         "{input.binary}  {input.active_sites} {input.trips} {input.vehicles} | grep -v '^Academic license' > {output}"

rule create_projection_to_larger_step_cross:
    input:
         script="preprocessing/project_between_stepped_sites.py",
         larger_set=find_taxi_step,
         current_active_sites=OUTPUT_PREFIX + "/cross/stepped{NUM_SITES,[0-9]+}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}cross_active_sites.csv"
    output:
          OUTPUT_PREFIX + "/cross/stepped{NUM_SITES,[0-9]+}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}active_sites_projected_to_larger_cross.csv"
    conda:
          "environment.yaml"
    shell:
         "python {input.script} {input.larger_set} {input.current_active_sites} > {output}"

rule selftest_feasibility_on_larger_step_cross:
    group: "solving_bucket"
    params:
          larger_num_sites=get_bigger_step
    input:
         active_sites=OUTPUT_PREFIX + "/cross/stepped{NUM_SITES,[0-9]+}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}active_sites_projected_to_larger_cross.csv",
         vehicles=lambda wildcards: expand(OUTPUT_PREFIX+ "/preprocessed/stepped{larger_num_sites}/{{time_bucket}}/{{BATTERY}}/tol{{TOLERANCE}}/final.vehicles.csv",larger_num_sites=get_bigger_step(wildcards)),
         trips=lambda wildcards: expand(OUTPUT_PREFIX +"/preprocessed/stepped{larger_num_sites}/{{time_bucket}}/{{BATTERY}}/tol{{TOLERANCE}}/final.trips.csv",larger_num_sites=get_bigger_step(wildcards)),
         binary=OUTPUT_PREFIX + "/binaries/check_feasibility"
    output:
          OUTPUT_PREFIX + "/cross/stepped{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}validate_self_against_larger_cross"
    shell:
         "{input.binary} {input.vehicles} {input.trips} {input.active_sites} > {output}"

