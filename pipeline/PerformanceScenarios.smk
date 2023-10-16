

rule get_performance_instance_stats:
    threads: 1
    conda:
         "environment.yaml"
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.vehicles.csv.gz",
        trips = OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.trips.csv.gz",
        script = "preprocessing/instance_stats.py"
    output:
        OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.stats.txt"
    shell:
        "python {input.script} {input.vehicles} {input.trips} > {output}"

rule run_opt_on_group:
    resources:
             runtime=lambda wildcards, attempt: 3 * (60 * 60) if int(wildcards.INT_NUM_SITES) <= 30 else 6 * (60*60), mem_mb=3000
    group: "solving_bucket"
    threads: 1
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.vehicles.csv.gz",
        sites=OUTPUT_PREFIX + "/preprocessed/{INT_NUM_SITES}{SUFFIX_DASH}.sites.csv",
        trips = OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.trips.csv.gz",
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        binary=OUTPUT_PREFIX + "/binaries/benders"
    output:
        stdout=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/opt_log",
        charge_process=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/opt_chargeprocess",
        cuts=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/opt_cuts"
    log:
        trace=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/opt_log_trace.bin"
    shell:
        "{input.binary}  --vehicles {input.vehicles} --trips {input.trips} --cuts_output {output.cuts}  --sites {input.sites} --battery {input.battery} --sites_min=1 --percent_infeasible_allowed 0.{wildcards.TOLERANCE} --charge_processes_file {output.charge_process} --hawktracer_output {log.trace} > {output.stdout}"


rule test_input_feasibilitygroup:
    resources:
             runtime=lambda wildcards, attempt: 3 * (60 * 60) if int(wildcards.INT_NUM_SITES) <= 30 else 6 * (60*60), mem_mb=3000
    group: "solving_bucket"
    threads: 1
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.vehicles.csv.gz",
        sites=OUTPUT_PREFIX + "/preprocessed/{INT_NUM_SITES}{SUFFIX_DASH}.sites.csv",
        trips = OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.trips.csv.gz",
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        binary=OUTPUT_PREFIX + "/binaries/check_feasibility"
    output:
        stdout=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/input_feasible",
    shell:
        "{input.binary}  --vehicles {input.vehicles} --trips {input.trips} --sites {input.sites} --battery {input.battery} > {output.stdout}"




rule run_var_run_opt_on_group:
    priority: -1# only do at the end since var gets stuck sometimes
    resources:
             runtime=lambda wildcards, attempt: 3 * (60 * 60) if int(wildcards.INT_NUM_SITES) <= 30 else 6 * (60*60), mem_mb=3000
    group: "solving_bucket"
    threads: 1
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.vehicles.csv.gz",
        sites=OUTPUT_PREFIX + "/preprocessed/{INT_NUM_SITES}{SUFFIX_DASH}.sites.csv",
        trips = OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.trips.csv.gz",
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        cuts=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/opt_cuts",
        binary=OUTPUT_PREFIX + "/binaries/bendersVar"
    output:
          stdout=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/var_opt_log",
          cuts=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/var_opt_cuts",
          charge_process=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/var_opt_chargeprocess"
    log:
       trace=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/var_opt_log_trace.bin"
    shell:
         "{input.binary}  --vehicles {input.vehicles} --trips {input.trips} --cuts_input {input.cuts} --cuts_output {output.cuts}  --sites {input.sites} --battery {input.battery} --sites_min=1 --percent_infeasible_allowed 0.{wildcards.TOLERANCE} --charge_processes_file {output.charge_process} --hawktracer_output {log.trace} > {output.stdout}"




rule extract_site_csv:
    group: "solving_bucket"
    input:
         script="preprocessing/extract_site_results.py",
         sites=OUTPUT_PREFIX + "/preprocessed/{NUM_SITES}.sites.csv",
         optlog=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{NUM_SITES}/{NUM_VEHICLES}/{is_var}{is_leveled}opt_log",
    output:
          OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{NUM_SITES}/{NUM_VEHICLES}/{is_var}{is_leveled}active_sites",
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.optlog} {input.sites} > {output}"