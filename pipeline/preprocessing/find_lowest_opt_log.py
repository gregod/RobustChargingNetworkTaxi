import re
import sys


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


min_cost = min(costs)
print("Min is",min_cost,"over",costs,file=sys.stderr)

print(log_files[costs.index(min_cost)])
