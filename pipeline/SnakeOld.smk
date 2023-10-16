configfile: "config.yaml"

OUTPUT_PREFIX = config["output_prefix"]

# individual days in data
days = range(1,30 + 1)

days_of_week = [0,1,2,3,4,5,6]

#M_1 = [ "2015-07-{:02}-00:00:00".format(day) for day in days]
#M_2 = [ "2015-08-{:02}-00:00:00".format(day) for day in days]
#M_3 = [ "2015-09-{:02}-00:00:00".format(day) for day in days]
#TIME_BUCKETS = M_1 + M_2 + M_3



import os
import re

include: "common.smk"
include: "CrossOpt.smk"
include: "Study.smk"
include: "Analysis.smk"
include: "NewIntegrated.smk"
include: "PerformanceScenarios.smk"

import getpass
if getpass.getuser() == "gu53rab2":
    # we are running in batch system; Make sure to load python and python environment before each run
    shell.prefix("module load python/3.6_intel gurobi;")
else:
    # if we are not on the batch system include rule to rebuild binaries and preprocessing on demand
    include: "BuildBinaries.smk"
    include: "Preprocessing.smk"
    ruleorder: generate_sample_from_group  > process_trips > remove_infeasible_vehicles


# run these rules always local instead of cluster
#localrules: collect_all_for_site_bucket, detect_bucket_finish, collect_bucket_results_per_site, all

GC_TIMELIMIT=14400



