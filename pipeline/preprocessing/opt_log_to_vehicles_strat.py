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

if "leveled_" in opt_log:
    reg = "(.*)/opt/(\d*)/group_(\d*)/battery_(\d*)/tol(\d\d)/([^/]*)/(\d*)/leveled_opt_log"
    groups = re.match(reg,opt_log).groups()
    print(groups[0] + f"/preprocessed/{groups[1]}/group_{groups[2]}/{groups[5]}/{groups[6]}/leveled.feasible.vehicles.csv.gz")
else:
    reg = "(.*)/opt/(\d*)/group_(\d*)/battery_(\d*)/tol(\d\d)/([^/]*)/(\d*)/opt_log"
    groups = re.match(reg,opt_log).groups()
    print(groups[0] + f"/preprocessed/{groups[1]}/group_{groups[2]}/{groups[6]}/{groups[5]}/battery_{groups[3]}.feasible.vehicles.csv.gz")


