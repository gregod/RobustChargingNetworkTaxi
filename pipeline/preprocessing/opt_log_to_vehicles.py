import pandas as pd
import re
import sys
import csv
import io
import sys

from statistics import  median
"""
Find the matching vehicle input file from the opt log
"""
import re
opt_log = sys.argv[1]


# work/opt/2/group_4/battery_1/50/variable/300/opt_log
reg = r"(.*)/opt/(\d*)/group_(\d*)/battery_(\d*)/([^/]*)/([^/]*)/(\d*)/opt_log"
groups = re.match(reg,opt_log).groups()

prefix = groups[0]
seed = groups[1]
group = groups[2]
battery = groups[3]
num_sites = groups[4]
site_size = groups[5]
num_vehicles = re.sub(r"[^\d]*", "", groups[6])

print(prefix + f"/preprocessed/{seed}/group_{group}/{num_sites}/{num_vehicles}/battery_{battery}.final.vehicles.csv.gz")

