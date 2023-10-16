rule run_variable_opt_on_group:
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
        binary=OUTPUT_PREFIX + "/binaries/benders_variable"
    output:
        stdout=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/variable_opt_log",
        charge_process=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/variable_opt_chargeprocess",
        cuts=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/variable_opt_cuts"
    log:
        trace=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/opt_log_trace.bin"
    shell:
        "{input.binary}  --vehicles {input.vehicles} --trips {input.trips} --cuts_input {input.cuts} --cuts_output {output.cuts}  --sites {input.sites} --battery {input.battery} --sites_min=1 --percent_infeasible_allowed 0.{wildcards.TOLERANCE} --charge_processes_file {output.charge_process} --hawktracer_output {log.trace} > {output.stdout}"


rule run_variable_opt_on_group_no_cuts:
    resources:
             runtime=lambda wildcards, attempt: 3 * (60 * 60) if int(wildcards.INT_NUM_SITES) <= 30 else 6 * (60*60), mem_mb=3000
    group: "solving_bucket"
    threads: 1
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.vehicles.csv.gz",
        sites=OUTPUT_PREFIX + "/preprocessed/{INT_NUM_SITES}{SUFFIX_DASH}.sites.csv",
        trips = OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.trips.csv.gz",
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        binary=OUTPUT_PREFIX + "/binaries/benders_variable"
    output:
        stdout=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/variable_no_cuts_opt_log",
        charge_process=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/variable_no_cuts_opt_chargeprocess",
        cuts=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/variable_no_cuts_opt_cuts"
    log:
        trace=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/opt_no_cuts_log_trace.bin"
    shell:
        "{input.binary}  --vehicles {input.vehicles} --trips {input.trips} --cuts_output {output.cuts}  --sites {input.sites} --battery {input.battery} --sites_min=1 --percent_infeasible_allowed 0.{wildcards.TOLERANCE} --charge_processes_file {output.charge_process} --hawktracer_output {log.trace} > {output.stdout}"