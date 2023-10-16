

rule generate_opt_utilisation_plot:
    threads: 1
    input:
         charge_process=OUTPUT_PREFIX + "/{cross_or_opt}/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}{is_cross}opt_chargeprocess",
         active_sites=OUTPUT_PREFIX + "/{cross_or_opt}/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}{is_cross}active_sites.csv",
         script="preprocessing/plot_site_utilisation.py"
    output:
          fig=OUTPUT_PREFIX + "/{cross_or_opt}/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}{is_cross}site_utilisation.png"
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.charge_process} {input.active_sites} {output.fig}"


rule generate_opt_bounds_plot:
    threads: 1
    input:
         opt_log=OUTPUT_PREFIX + "/{cross_or_opt}/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}{is_cross}opt_log",
         script="preprocessing/plot_bounds.py"
    output:
          figPng=OUTPUT_PREFIX + "/{cross_or_opt}/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}{is_cross}opt_bounds.png",
          figTex=OUTPUT_PREFIX + "/{cross_or_opt}/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}{is_cross}opt_bounds.tikz",
          figPngTime=OUTPUT_PREFIX + "/{cross_or_opt}/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}{is_cross}opt_bounds_time.png",
          figTexTime=OUTPUT_PREFIX + "/{cross_or_opt}/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}{is_cross}opt_bounds_time.tikz"
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.opt_log} {output.figPng} {output.figTex} {output.figPngTime} {output.figTexTime}"

# TESTS


rule create_projection_to_larger_step:
    input:
         script="preprocessing/project_between_stepped_sites.py",
         larger_set=find_taxi_step,
         current_active_sites=OUTPUT_PREFIX + "/opt/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/active_sites.csv"
    output:
          OUTPUT_PREFIX + "/opt/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/active_sites_projected_to_larger.csv"
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.larger_set} {input.current_active_sites} > {output}"


rule selftest_feasibility_on_larger_step:
    group: "solving_bucket"
    params:
          larger_num_sites=get_bigger_step
    input:
         active_sites=OUTPUT_PREFIX + "/opt/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/active_sites_projected_to_larger.csv",
         vehicles=lambda wildcards: expand(OUTPUT_PREFIX+ "/preprocessed/{larger_num_sites}/{{time_bucket}}/{{BATTERY}}/tol{{TOLERANCE}}/final.vehicles.csv",larger_num_sites=get_bigger_step(wildcards)),
         trips=lambda wildcards: expand(OUTPUT_PREFIX +"/preprocessed/{larger_num_sites}/{{time_bucket}}/{{BATTERY}}/tol{{TOLERANCE}}/final.trips.csv",larger_num_sites=get_bigger_step(wildcards)),
         battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
         binary=OUTPUT_PREFIX + "/binaries/check_feasibility"
    output:
          OUTPUT_PREFIX + "/opt/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/validate_self_against_larger"
    shell:
         "{input.binary} --vehicles {input.vehicles} --trips {input.trips} --sites {input.active_sites} --battery {input.battery} > {output}"

rule selftest_feasibility:
    group: "solving_bucket"
    input:
         active_sites=OUTPUT_PREFIX + "/opt/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}active_sites.csv",
         vehicles=OUTPUT_PREFIX+ "/preprocessed/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/final.vehicles.csv",
         trips=OUTPUT_PREFIX +"/preprocessed/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/final.trips.csv",
         battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
         binary=OUTPUT_PREFIX + "/binaries/check_feasibility"
    output:
          OUTPUT_PREFIX + "/opt/{NUM_SITES}/{time_bucket}/{BATTERY}/tol{TOLERANCE}/{is_var}validate_self"
    shell:
         "{input.binary} --vehicles {input.vehicles} --trips {input.trips} --sites {input.active_sites} --battery {input.battery} > {output}"

