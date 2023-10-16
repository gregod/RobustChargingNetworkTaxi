OUTPUT_PREFIX="/mnt/dataHDD/split_days"


# individual days in data
days = range(1,30 + 1)

# which days to extract from. 0=Mittwoch,1=Donnerstag
days_of_week = [0,1,2,3,4,5,6]

TIME_BUCKETS = [ "2015-07-{:02}-00:00:00".format(day) for day in days]



import getpass
if getpass.getuser() == "gu53rab2":
    # we are running in batch system; Make sure to load python and python environment before each run
    shell.prefix("module load python;export LD_LIBRARY_PATH=$HOME/pipeline/; ")
    OUTPUT_PREFIX="/home/hpc/tf141/gu53rab2/split_days"
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
#         OUTPUT_PREFIX + "/10/final",
#         OUTPUT_PREFIX + "/20/final",
        OUTPUT_PREFIX + "/30/2015-07-12-00:00:00/active_sites.csv",
        OUTPUT_PREFIX + "/30/2015-07-13-00:00:00/active_sites.csv",
        OUTPUT_PREFIX + "/30/virtual-1",
        OUTPUT_PREFIX + "/60/2015-07-12-00:00:00/active_sites.csv",
        OUTPUT_PREFIX + "/60/2015-07-13-00:00:00/active_sites.csv",
        OUTPUT_PREFIX + "/60/virtual-1",




rule final:
    resources:
        runtime=30
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


rule virtual:
    resources:
             runtime=30
    group : "output"
    input:
         lambda wildcards: expand(
             os.path.join(OUTPUT_PREFIX,"{{NUM_SITES}}","virtual_{time_bucket}-{{virtual_len}}","active_sites.csv"),
             time_bucket=days_of_week
         )
    output:
          OUTPUT_PREFIX + "/{NUM_SITES}/virtual-{virtual_len}"
    shell:
         "cat {input} > {output}"


rule run_bin_batch_gc_on_site_bucket:
    resources:
        runtime=lambda a: int(GC_TIMELIMIT * 1.1), mem_mb=2048
    group: "solving_bucket"
    threads: 2
    input:
        vehicles=OUTPUT_PREFIX+ "/{NUM_SITES}/{time_bucket}/final.vehicles.csv",
        sites=OUTPUT_PREFIX + "/{NUM_SITES}.sites.csv",
        trips=OUTPUT_PREFIX +"/{NUM_SITES}/{time_bucket}/final.trips.csv",
        binary=OUTPUT_PREFIX + "/{NUM_SITES}/batch_gc"
    output:
        OUTPUT_PREFIX + "/{NUM_SITES}/{time_bucket}/batch_gc_log"
    shell:
        "{input.binary} --vehicles {input.vehicles} --trips {input.trips}  --sites {input.sites} --sites_min=7  > {output}"

rule run_bin_variable_on_site_bucket:
    resources:
             runtime=lambda a: int(GC_TIMELIMIT * 1.1), mem_mb=2048
    group: "solving_bucket"
    threads: 1
    input:
         vehicles=OUTPUT_PREFIX+ "/{NUM_SITES}/{time_bucket}/final.vehicles.csv",
         sites=OUTPUT_PREFIX + "/{NUM_SITES}.sites.csv",
         trips=OUTPUT_PREFIX +"/{NUM_SITES}/{time_bucket}/final.trips.csv",
         binary=OUTPUT_PREFIX + "/{NUM_SITES}/variable"
    output:
          OUTPUT_PREFIX + "/{NUM_SITES}/{time_bucket}/variable_log"
    shell:
         "{input.binary} --vehicles {input.vehicles} --trips {input.trips}  --sites {input.sites} --sites_min=7 --sites_max=10 --perf-CutWhenRollingAvgExeeding --perf-Probing  --perf-CutSmallerInfeasibleGroup > {output}"



rule extract_site_csv:
    group: "solving_bucket"
    input:
        script="preprocessing/extract_site_results.py",
        sites=OUTPUT_PREFIX + "/{NUM_SITES}.sites.csv",
        optlogA=OUTPUT_PREFIX + "/{NUM_SITES}/{time_bucket}/batch_gc_log",
        optlogB=OUTPUT_PREFIX + "/{NUM_SITES}/{time_bucket}/variable_log",
    output:
        OUTPUT_PREFIX + "/{NUM_SITES}/{time_bucket}/active_sites.csv"
    conda:
        "environment.yaml"
    shell:
        "python {input.script} {input.optlogA} {input.sites} > {output}"
         "; python {input.script} {input.optlogB} {input.sites} >> {output}"



