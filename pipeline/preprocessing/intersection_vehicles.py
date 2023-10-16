#!/usr/bin/env python
# coding: utf-8

"""
Generates intersection between vehicle input files
(Only vehicles that appear in all of them)

Arguments:  vehicle_input_file [...]
"""

import pandas as pd
import sys
import csv
import io



output = io.StringIO()



inputs = sys.argv[1:]
all_sets = []
for file in inputs:
    df = pd.read_csv(file)
    df.index = df["index"]
    all_sets.append(set(df["id"]))

intersect = set.intersection(*all_sets)

filtered_df = (df[df["id"].isin(intersect)])
filtered_df.to_csv(path_or_buf=output, quoting=csv.QUOTE_MINIMAL, index=False)
print(output.getvalue(),end="")