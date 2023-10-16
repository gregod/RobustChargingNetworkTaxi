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


# work/opt/2/group_4/battery_1/50/variable/300/opt_log
reg = r"(.*)/opt/(\d*)/group_(\d*)/battery_(\d*)/([^/]*)/([^/]*)/(\d*)/opt_log"
groups = re.match(reg,opt_log).groups()
print(groups[0] + f"/preprocessed/{groups[1]}/group_{groups[2]}/{groups[6]}/{groups[5]}/fixed.trips.csv.gz")