import pandas as pd
import re
import sys
import csv
import io
import sys

from statistics import  median
"""
Find the matching opt cut file from the opt log
"""
opt_log = sys.argv[1].replace("opt_log","opt_cuts")
print(opt_log)