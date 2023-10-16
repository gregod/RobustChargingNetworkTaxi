
import re
import sys
import numpy

log_files = sys.argv[1:]

costs = []
for log_file in log_files:
    with open(log_file, "r") as file:
        the_line = None

        for line in file:
            match = re.search("^Best cost (\d*)", line)
            if match:
                the_line = match.group(1)
                # do not break, we want to find the last row
        if the_line:
            costs.append(int(the_line))

# median for even length lists is avg of middle values,
# this approximates it by sorting index, and picking middle
med_index = numpy.argsort(costs)[len(costs)//2]
med_costs = costs[med_index]

print("Median is",med_costs,"over",costs,file=sys.stderr)

print(log_files[med_index])
