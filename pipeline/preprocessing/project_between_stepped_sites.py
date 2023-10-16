import pandas as pd
import re
import sys
import csv
import io

'''
Projects an small site file to larger case.
All sites that exist in both are set to the
site sizes from the smaller file.
All others are set to 0. 
Should have same feasibility and costs with stepped
sites!
'''
_, large_site_file, small_site_file = sys.argv




large_sites = pd.read_csv(large_site_file)
large_sites.index = large_sites["id"]
large_sites["capacity"] = 0

small_sites = pd.read_csv(small_site_file)
small_sites.index = small_sites["id"]

large_sites["capacity"] = small_sites["capacity"]
large_sites["capacity"] = large_sites["capacity"].fillna(0).astype(int)


output = io.StringIO()
spamwriter = csv.writer(output, delimiter=',',
                        quotechar='"', quoting=csv.QUOTE_NONNUMERIC
                        )
spamwriter.writerow(['id', 'capacity', 'cost', 'location'])
for index,row in large_sites.iterrows():
    spamwriter.writerow([row["id"],row["capacity"],row["cost"],row["location"]])

print(output.getvalue())