OUTPUT_PREFIX="/mnt/dataHDD/split_days"


# individual days in data
days = range(1,30 + 1)

# which days to extract from. 0=Mittwoch,1=Donnerstag
days_of_week = [0,1,2,3,4,5,6]

TIME_BUCKETS = [ "2015-07-{:02}-00:00:00".format(day) for day in days]

import os


import getpass
if getpass.getuser() == "gu53rab2":
    # we are running in batch system; Make sure to load python and python environment before each run
    shell.prefix("module load python/3.6_intel gurobi;")
    OUTPUT_PREFIX=os.environ['HOME'] + "/newpipe/output"
else:
    # if we are not on the batch system include rule to rebuild binaries on demand
    include: "BuildBinaries.smk"

# run these rules always local instead of cluster
#localrules: collect_all_for_site_bucket, detect_bucket_finish, collect_bucket_results_per_site, all
ruleorder: create_virtual_site_buckets > process_trips > remove_infeasible_vehicles

SPEEDS=[0.001414]
GC_TIMELIMIT=14400

wildcard_constraints:
    NUM_SITES="\d+",
    time_bucket="(\d\d\d\d-\d\d-\d\d-\d\d:\d\d:\d\d)|(virtual_\d*-\d*)"

include: "Preprocessing.smk"

rule all:
    input:
        OUTPUT_PREFIX + "/30/virtual_1-2/active_sites_1.csv",
        OUTPUT_PREFIX + "/30/virtual_2-2/active_sites_1.csv",
        OUTPUT_PREFIX + "/30/virtual_3-2/active_sites_1.csv",
        OUTPUT_PREFIX + "/30/virtual_4-2/active_sites_1.csv",
        OUTPUT_PREFIX + "/30/virtual_5-2/active_sites_1.csv",
        OUTPUT_PREFIX + "/30/virtual_6-2/active_sites_1.csv",
        OUTPUT_PREFIX + "/60/virtual_1-2/active_sites_1.csv",
        OUTPUT_PREFIX + "/60/virtual_2-2/active_sites_1.csv",
        OUTPUT_PREFIX + "/60/virtual_3-2/active_sites_1.csv",
        OUTPUT_PREFIX + "/60/virtual_4-2/active_sites_1.csv",
        OUTPUT_PREFIX + "/60/virtual_5-2/active_sites_1.csv",
        OUTPUT_PREFIX + "/60/virtual_6-2/active_sites_1.csv",




rule final:
    resources:
        runtime=30, mem_mb=512
    group : "output"
    input:
         lambda wildcards: expand(
             os.path.join(OUTPUT_PREFIX,"{{NUM_SITES}}","{time_bucket}","active_sites.csv"),
             time_bucket=TIME_BUCKETS
         )
    output:
          OUTPUT_PREFIX + "/{NUM_SITES}/final"
    shell:
        "cat {input} > {output}"



rule run_bin_benders_on_site_bucket:
    group: "solving_bucket"
    resources:
         runtime=lambda wildcards, attempt: 3 * (60 * 60) if int(wildcards.NUM_SITES) == 30 else 40 * (60*60), mem_mb=8129
    threads: 1
    input:
        vehicles=OUTPUT_PREFIX+ "/{NUM_SITES}/{time_bucket}/final.vehicles.csv",
        sites=OUTPUT_PREFIX + "/{NUM_SITES}.sites.csv",
        trips=OUTPUT_PREFIX +"/{NUM_SITES}/{time_bucket}/final.trips.csv",
        binary=OUTPUT_PREFIX + "/{NUM_SITES}/benders"
    output:
        OUTPUT_PREFIX + "/{NUM_SITES}/{time_bucket}/benders_log_{sites_min}"
    shell:
        "{input.binary} --vehicles {input.vehicles} --trips {input.trips}  --sites {input.sites} --sites_min={wildcards.sites_min}  > {output}"


rule extract_site_csv:
    group: "solving_bucket"
    input:
        script="preprocessing/extract_site_results.py",
        sites=OUTPUT_PREFIX + "/{NUM_SITES}.sites.csv",
        optlogA=OUTPUT_PREFIX + "/{NUM_SITES}/{time_bucket}/benders_log_{sites_min}",
    output:
        OUTPUT_PREFIX + "/{NUM_SITES}/{time_bucket}/active_sites_{sites_min}.csv"
    conda:
        "environment.yaml"
    shell:
        "python {input.script} {input.optlogA} {input.sites} > {output}"



