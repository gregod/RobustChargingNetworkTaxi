"""
Takes a list of sites and updates their costs based on a distance formula
"""
import csv
import numpy as np
import pandas as pd
import sys
from math import radians, cos, sin, asin, sqrt,exp
import io


import random

output = io.StringIO()

_, input_file = sys.argv

df = pd.read_csv(input_file)
df.index = df["id"]

# set the cost to the rounded mean costs of the "normal cost strategy"
df["cost"] = int(round(df["cost"].mean()))


spamwriter = csv.writer(output, delimiter=',',
                        quotechar='"', quoting=csv.QUOTE_NONNUMERIC
                        )
spamwriter.writerow(['id', 'capacity', 'cost', 'location'])

for index,row in df.iterrows():
    spamwriter.writerow([row["id"],row["capacity"],row["cost"],row["location"]])

print(output.getvalue())

