
rule simulation_feasible:
    resources:
             runtime=lambda wildcards, attempt: 3 * (60 * 60) if int(wildcards.INT_NUM_SITES) <= 30 else 6 * (60*60), mem_mb=3000
    group: "solving_bucket"
    threads: 1
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.vehicles.csv.gz",
        active_sites=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/{INT_NUM_SITES}/{SITE_SIZE}/{NUM_VEHICLES}/active_sites",
        trips = OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.trips.csv.gz",
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        binary=OUTPUT_PREFIX + "/binaries/simulation_feasible"
    output: OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/{INT_NUM_SITES}/{SITE_SIZE}/{NUM_VEHICLES}/simulation_feasible"
    shell:
        "{input.binary} --vehicles {input.vehicles} --trips {input.trips} --sites {input.active_sites} --battery {input.battery} > {output}"





rule simulation_feasible_robust:
    resources:
             runtime=lambda wildcards, attempt: 3 * (60 * 60) if int(wildcards.INT_NUM_SITES) <= 30 else 6 * (60*60), mem_mb=3000
    group: "solving_bucket"
    threads: 1
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.vehicles.csv.gz",
        
        active_sites=OUTPUT_PREFIX + "/opt/robust/{BATTERY}/{INT_NUM_SITES}/{SITE_SIZE}/{NUM_VEHICLES}/lowest_active_sites_quorum:100_activate:1_benevolent:5_iis:true",


        trips = OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.trips.csv.gz",
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        binary=OUTPUT_PREFIX + "/binaries/simulation_feasible"
    output: OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/{INT_NUM_SITES}/{SITE_SIZE}/{NUM_VEHICLES}/simulation_feasible_robust"
    shell:
        "{input.binary} --vehicles {input.vehicles} --trips {input.trips} --sites {input.active_sites} --battery {input.battery} > {output}"

