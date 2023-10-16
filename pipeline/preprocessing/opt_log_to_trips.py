import pandas as pd
import re
import sys
import csv
import io
import sys

from statistics import  median
"""
Find the matching trip input file from the opt log
"""
import re
opt_log = sys.argv[1]

if "leveled_" in opt_log:
    reg = "(.*)/opt/(\d*)/group_(\d*)/battery_(\d*)/tol(\d\d)/([^/]*)/(\d*)/leveled_opt_log"
    groups = re.match(reg,opt_log).groups()
    num_vehicles = re.sub(r"[^\d]*", "", groups[5])
    print(groups[0] + f"/preprocessed/{groups[1]}/group_{groups[2]}/{num_vehicles}/{groups[6]}/leveled.final.trips.csv.gz")
else:
    reg = "(.*)/opt/(\d*)/group_(\d*)/battery_(\d*)/tol(\d\d)/([^/]*)/(\d*)/opt_log"
    groups = re.match(reg,opt_log).groups()
    num_vehicles = re.sub(r"[^\d]*", "", groups[5])
    print(groups[0] + f"/preprocessed/{groups[1]}/group_{groups[2]}/{num_vehicles}/{groups[6]}/battery_{groups[3]}.final.trips.csv.gz")