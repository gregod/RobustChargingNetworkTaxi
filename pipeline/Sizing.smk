configfile: "config.yaml"
wildcard_constraints:
    INT_NUM_SITES="\d*", #  int number for site
    NUM_SITES="\d*(|_\w*)", #  number for site
    SUFFIX_DASH="(|_\w*)",

OUTPUT_PREFIX = "work_sizing"
INPUT_PREFIX = "work"


rule main:
    input:
        expand( OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol00/{NUM_SITES}/{NUM_VEHICLES}/{SIZING}/opt_log",
            SEED=[1],
            TYPE_GROUP=[1],
            BATTERY=["battery_1"],
            NUM_SITES=[30],
            NUM_VEHICLES=[100,500,1000],
            SIZING=["2-2","4-2","4-4"],
        )


rule run_opt_on_group:
    group: "solving_bucket"
    threads: 1
    input:
        vehicles=INPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.vehicles.csv.gz",
        sites=INPUT_PREFIX + "/preprocessed/{INT_NUM_SITES}{SUFFIX_DASH}.sites.csv",
        trips = INPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.trips.csv.gz",
        battery=INPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        binary=OUTPUT_PREFIX + "/binaries/benders"
    output:
        stdout=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/{SIZE_START}-{SIZE_FINAL}/opt_log",
        charge_process=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/{SIZE_START}-{SIZE_FINAL}/opt_chargeprocess",
        cuts=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/{SIZE_START}-{SIZE_FINAL}/opt_cuts"
    log:
        trace=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/{SIZE_START}-{SIZE_FINAL}/opt_log_trace.bin",
        stdout=OUTPUT_PREFIX + "/opt/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/{SIZE_START}-{SIZE_FINAL}/opt_log.log",
    shell:
        "{input.binary}  --vehicles {input.vehicles} --trips {input.trips} --cuts_output {output.cuts}  --sites {input.sites} --battery {input.battery} --percent_infeasible_allowed 0.{wildcards.TOLERANCE} --static_station_size={wildcards.SIZE_FINAL} --initial_station_size={wildcards.SIZE_START} --charge_processes_file {output.charge_process} --hawktracer_output {log.trace} | tee {log.stdout} > {output.stdout}"





rule build_benders_binary:
    threads: 1
    resources:
             cargo=1
    input:
         cargo="/home/gregor/Code/et/column_generation/src/bin/benders.rs"
    output:
          OUTPUT_PREFIX + "/binaries/benders"
    shell:
         "RUSTFLAGS='-C link-arg=-s -C target-cpu=native' cargo +nightly build -Z unstable-options -j 1 --profile=cluster --features='snakemake,profiling_enabled' --manifest-path /home/gregor/Code/et/column_generation/Cargo.toml --no-default-features --bin benders --target-dir {OUTPUT_PREFIX}/target/ --out-dir $(dirname {output})"
