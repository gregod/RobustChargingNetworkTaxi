import pandas as pd
import re
import sys
import csv
import io


_, log_file, site_file = sys.argv

file = open(log_file, "r")

the_line = ""

for line in file:
    if re.search("^Best cost \d", line):
        the_line = line
        # do not break, we want to find the last row

line_vector = [ int(v) for v in the_line[the_line.find("[")+1:-2].split(",")]

sites = pd.read_csv(site_file)
sites["capacity"] = line_vector[:len(sites)] # cut of any values larger than num sites; (All Zeros for technical reasons)



output = io.StringIO()
csvwriter = csv.writer(output, delimiter=',',
                       quotechar='"', quoting=csv.QUOTE_NONNUMERIC
                       )
csvwriter.writerow(['id', 'capacity', 'cost', 'location', 'lat', 'lon'])

for index,row in sites.iterrows():
    location = row["location"][1:-1].split(", ")
    csvwriter.writerow([row["id"], row["capacity"], row["cost"], row["location"], float(location[0]), float(location[1])])

print(output.getvalue())
