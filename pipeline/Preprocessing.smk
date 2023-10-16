ruleorder: generate_sample_from_feasible_group  > process_trips > resample_capacity_infeasible_vehicles

# Look at the input data and form the input data type groups

# individual days in data
from datetime import date, timedelta
sdate = date(2015, 3, 5)   # start date
edate = date(2015, 9, 28)   # end date
delta = edate - sdate       # as timedelta
all_days_for_groups = [sdate + timedelta(days=i) for i in range(delta.days + 1)]



rule split_vehicles_into_time_buckets:
    resources:
             runtime=1800
    group: "preprocessing"
    input:
         script="preprocessing/split_vehicles.py",
         vehicles="input_data/vehicles.csv.gz",
         trips="input_data/trips.csv.gz",
    output:
          vehicles=expand(OUTPUT_PREFIX+ "/preprocessed/vehicles/{time_bucket}.csv.gz", time_bucket=[d.strftime("%Y-%m-%d-00:00:00") for d in all_days_for_groups]),
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.vehicles} {input.trips} " + OUTPUT_PREFIX + "/preprocessed/vehicles"

#Takes a vehicle and trip file and transforms start and endtimes to periods
#Preprocessing is applied to make sure that periods do not overlap
rule fix_trips:
    group: "preprocessing"
    resources:
             runtime=3600
    input:
         script="preprocessing/fix_trips.py",
         trips="input_data/trips.csv.gz",
         vehicles=OUTPUT_PREFIX+ "/preprocessed/vehicles/{time_bucket}.csv.gz",
    output:
          taxi_trips=OUTPUT_PREFIX + "/preprocessed/{time_bucket}/taxi_trips_fixed.csv.gz",
          vehicles=OUTPUT_PREFIX + "/preprocessed/{time_bucket}/vehicles.csv.gz",
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.vehicles} {input.trips}  {output.vehicles} | gzip > {output.taxi_trips}"



# extract input days based on wildcard.typegroup
# currently this is the weekday
def get_dates_by_type_group(type_group):
    weekday = type_group
    return [d for d in all_days_for_groups if d.weekday() == weekday]

def get_vehicles_by_type_group(wildcards):
    weekday = int(wildcards.TYPE_GROUP)
    dates = get_dates_by_type_group(weekday)
    return expand(
        os.path.join(OUTPUT_PREFIX,"preprocessed","{time_bucket}", "vehicles.csv.gz"),
        time_bucket=[d.strftime("%Y-%m-%d-00:00:00") for d in dates]
    )

def get_trips_by_type_group(wildcards):
    weekday = int(wildcards.TYPE_GROUP)
    dates = get_dates_by_type_group(weekday)
    return expand(
        os.path.join(OUTPUT_PREFIX,"preprocessed","{time_bucket}","taxi_trips_fixed.csv.gz"),
        time_bucket=[d.strftime("%Y-%m-%d-00:00:00") for d in dates]
    )


rule generate_day_groups:
    group: "preprocessing"
    conda:
         "environment.yaml"
    input:
        script="preprocessing/merge_cases.py",
        vehicles=get_vehicles_by_type_group,
        trips=get_trips_by_type_group
    output:
        vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/vehicles.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/taxi_trips_fixed.csv.gz"
    shell:
        "python {input.script} {input.vehicles} {input.trips} {output.vehicles} {output.trips}"



# take the sites and trips and produce file that contains the trips with reachable sites.
rule precompute_distance_matrix_for_trips:
    resources:
             runtime=1800, mem_mb=15000
    group: "preprocessing"
    threads: 1
    input:
         script="preprocessing/build_distance_cache.jl",
         taxi_trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/taxi_trips_fixed.csv.gz",
         taxi_sites="input_data/taxi_sites.csv",
    output:
          OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/distance_cache.jls"
    shell:
         "julia --project=. -t {threads} {input.script} --sites {input.taxi_sites} --trips {input.taxi_trips} --output {output}"


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
#rule process_trips:
#    resources:
#             runtime=1800, mem_mb=14000
#    group: "preprocessing"
#    threads: 3
#    input:
#         script="preprocessing/process_trips.py",
#         taxi_trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/taxi_trips_fixed.csv.gz",
#         taxi_sites=OUTPUT_PREFIX + "/preprocessed/nocost_{NUM_SITES}.sites.csv"
#    output:
#          OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/final.trips.csv.gz"
#    conda:
#         "environment.yaml"
#    shell:
#         "python {input.script} {input.taxi_sites}  {input.taxi_trips} 0.0017 | gzip > {output}"

# take the sites and trips and produce file that contains the trips with reachable sites.
rule process_trips:
    resources:
             runtime=1800, mem_mb=8000
    input:
         script="preprocessing/process_trips_orig.jl",
         taxi_trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/fall.processed.trips.csv.gz",
         taxi_sites=OUTPUT_PREFIX + "/preprocessed/nocost_{NUM_SITES}.sites.csv",
    output:
         OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/final.trips.csv.gz"
    shell:
         "julia --project=. {input.script} --sites {input.taxi_sites} --trips {input.taxi_trips} --output {output}"



rule determine_time_distributions_for_group:
    group: "preprocessing"
    conda:
         "environment.yaml"
    input:
        script="preprocessing/determine_distribution.py",
        vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{DOT_LEVELED}vehicles.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/taxi_trips_fixed.csv.gz",
    output:
        dist=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{DOT_LEVELED}distribution.txt"
    shell:
        "python {input.script} {input.vehicles} {input.trips} > {output.dist}"



rule cap_trip_length:
    group: "preprocessing"
    conda:
         "environment.yaml"
    input:
        script="preprocessing/cap_vehicle_duration.py",
        vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/vehicles.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/taxi_trips_fixed.csv.gz"
    output:
        vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/capped_{CAP}.vehicles.csv.gz",
    shell:
        "python {input.script} {input.vehicles} {input.trips} {wildcards.CAP} | gzip > {output.vehicles}"


leveled_bat=config["leveled"]["batteries"]
leveled_sites=config["leveled"]["sites"]


rule ensure_feasible_for_config:
    group: "preprocessing_feasible_sample"
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/capped_8.vehicles.csv.gz",
        # retain only the digits for nocost!
        trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/final.trips.csv.gz",
        sites=OUTPUT_PREFIX + "/preprocessed/nocost_{NUM_SITES}.sites.csv",
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        binary=OUTPUT_PREFIX + "/binaries/remove_infeasible"
    output:
        vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/{BATTERY}.feasible.vehicles.csv.gz"
    shell:
        "{input.binary} --vehicles {input.vehicles} --trips {input.trips} --sites {input.sites} --battery {input.battery} | gzip  > {output.vehicles}"


rule ensure_feasible_for_config_count:
    group: "preprocessing_2"
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/capped_8.vehicles.csv.gz",
        # retain only the digits for nocost!
        trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/final.trips.csv.gz",
        sites=OUTPUT_PREFIX + "/preprocessed/nocost_{NUM_SITES}.sites.csv",
        battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
        binary=OUTPUT_PREFIX + "/binaries/remove_infeasible"
    output:
        OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/{BATTERY}.feasible.vehicles.csv.gz.count"
    shell:
        "{input.binary} --vehicles {input.vehicles} --trips {input.trips} --sites {input.sites} --battery {input.battery} | wc -l  > {output}"



rule generate_leveled:
    group: "preprocessing"
    conda:
         "environment.yaml"
    input:
         vehicles=expand(OUTPUT_PREFIX + "/preprocessed/group_{{TYPE_GROUP}}/{NUM_SITES}/{BATTERY}.feasible.vehicles.csv.gz",NUM_SITES=leveled_sites,BATTERY=leveled_bat),
         script="preprocessing/intersection_vehicles.py"
    output:
         vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/leveled.vehicles.csv.gz"
    shell:
        "python {input.script} {input.vehicles} | gzip > {output.vehicles}"


rule generate_sample_from_feasible_group:
    group: "preprocessing_feasible_sample"
    conda:
         "environment.yaml"
    input:
        script="preprocessing/generate_sample.py",
        vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/{BATTERY}.feasible.vehicles.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/final.trips.csv.gz",
        dist=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/distribution.txt",
    output:
        vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.vehicles.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.sampled.trips.csv.gz"
    shell:
        "python {input.script} {input.vehicles} {input.trips} {input.dist} {wildcards.NUM_VEHICLES} {wildcards.SEED} {output.vehicles} {output.trips}"






rule generate_leveled_sample_from_group:
    group: "preprocessing"
    conda:
         "environment.yaml"
    input:
        script="preprocessing/generate_sample.py",
        vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/leveled.vehicles.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/final.trips.csv.gz",
        dist=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/leveled.distribution.txt",
    output:
        vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/leveled.final.vehicles.csv.gz",
        trips=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/leveled.final.trips.csv.gz"
    shell:
        "python {input.script} {input.vehicles} {input.trips} {input.dist} {wildcards.NUM_VEHICLES} {wildcards.SEED} {output.vehicles} {output.trips}"


rule gen_sites_full:
    group: "preprocessing"
    resources:
             runtime=3600
    input:
         script="preprocessing/process_sites.py",
         taxi_sites="input_data/taxi_sites.csv"
    output:
          OUTPUT_PREFIX + "/preprocessed/nocost_{INT_NUM_SITES,[0-9]+}.sites.csv"
    conda:
         "environment.yaml"
    shell:
         "python {input.script} {input.taxi_sites} {wildcards.INT_NUM_SITES} > {output}"



rule add_costs_using_function:
    group: "preprocessing"
    conda:
         "environment.yaml"
    resources:
             runtime=3600
    input:
         script="preprocessing/set_site_costs.py",
         taxi_sites=OUTPUT_PREFIX + "/preprocessed/nocost_{INT_NUM_SITES,[0-9]+}.sites.csv"
    output:
        taxi_sites=OUTPUT_PREFIX + "/preprocessed/{INT_NUM_SITES,[0-9]+}.sites.csv"
    shell:
         "python {input.script} {input.taxi_sites} > {output}"


rule add_costs_using_samecosts:
    group: "preprocessing"
    conda:
         "environment.yaml"
    resources:
             runtime=3600
    input:
         script="preprocessing/set_site_costs_samecosts.py",
         taxi_sites=OUTPUT_PREFIX + "/preprocessed/{INT_NUM_SITES,[0-9]+}.sites.csv"
    output:
        taxi_sites=OUTPUT_PREFIX + "/preprocessed/{INT_NUM_SITES,[0-9]+}_samecosts.sites.csv"
    shell:
         "python {input.script} {input.taxi_sites} > {output}"


rule get_battery:
    group: "preprocessing"
    conda:
         "environment.yaml"
    resources:
        runtime=3600
    input:
        battery="input_data/{BATTERY}.toml",
        script="preprocessing/generate_charging_function.py"
    output:
        battery=OUTPUT_PREFIX + "/preprocessed/{BATTERY}.toml"
    shell:
        "python {input.script} {input.battery} > {output.battery}"



# resample infeasible
#rule remove_infeasible_vehicles:
#    group: "preprocessing"
#    resources:
#             runtime=3600
#    input:
#         trips= OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.trips.csv.gz",
#         vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.vehicles.csv.gz",
#
#         presample_vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/{BATTERY}.feasible.vehicles.csv.gz",
#         presample_trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/final.trips.csv.gz",
#
#
#         # retain only the digits for nocost!
#         sites=OUTPUT_PREFIX + "/preprocessed/nocost_{NUM_SITES}.sites.csv",
#         battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
#         binary=OUTPUT_PREFIX + "/binaries/remove_infeasible"
#    output:
#          vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.vehicles.csv.gz",
#    shell:
#         "{input.binary} --vehicles {input.vehicles} --trips {input.trips} --sites {input.sites} --battery {input.battery} | gzip  > {output.vehicles}"


rule resample_capacity_infeasible_vehicles:
    group: "preprocessing_2"
    conda:
         "environment.yaml"
    resources:
             runtime=3600
    input:
         trips= OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.sampled.trips.csv.gz",
         vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.vehicles.csv.gz",

         presample_vehicles=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/{BATTERY}.feasible.vehicles.csv.gz",
         presample_trips=OUTPUT_PREFIX + "/preprocessed/group_{TYPE_GROUP}/{NUM_SITES}/final.trips.csv.gz",


         # retain only the digits for nocost!
         sites=OUTPUT_PREFIX + "/preprocessed/nocost_{NUM_SITES}.sites.csv",
         battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
         binary=OUTPUT_PREFIX + "/binaries/check_feasibility",
         script= "preprocessing/resample_till_feasible.py"
    output:
          vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.vehicles.csv.gz",
          trips=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.trips.csv.gz",
    shell:
        "python {input.script} {input.vehicles} {input.trips} {input.binary} {input.presample_vehicles} {input.presample_trips} {input.sites} {input.battery} {wildcards.SEED} {output.vehicles} {output.trips}"





rule do_required_for_opt:
    input:
       vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.vehicles.csv.gz",
       sites=OUTPUT_PREFIX + "/preprocessed/{NUM_SITES}.sites.csv",
       trips = OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.trips.csv.gz",
       battery=OUTPUT_PREFIX +"/preprocessed/{BATTERY}.toml",
    output:
          OUTPUT_PREFIX +"/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}/did_preprocessing"
    shell:
         "echo 1 > {output}"

rule do_required_for_leveled_opt:
    input:
        vehicles=OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/leveled.final.vehicles.csv.gz",
        sites=OUTPUT_PREFIX + "/preprocessed/{NUM_SITES}.sites.csv",
        trips = OUTPUT_PREFIX + "/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/leveled.final.trips.csv.gz",
    output:
          OUTPUT_PREFIX +"/preprocessed/{SEED}/group_{TYPE_GROUP}/{NUM_SITES}/{NUM_VEHICLES}/{BATTERY}/leveled_did_preprocessing"
    shell:
         "echo 1 > {output}"
