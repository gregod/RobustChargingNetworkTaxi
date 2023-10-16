import re
import sys

"""
Find the matching vehicle input file from the opt log
"""
import re
opt_log = sys.argv[1]
#opt_log = "/work/opt/realcase/1/dbat:1.00_dcha:1.00_dfin:1.00/opt_log"
# >> vehicles=OUTPUT_PREFIX + "/preprocessed/realcase/vehicles.base.dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.{DAY}.csv.gz",

reg = "(.*)/opt/realcase/(\d*)/dbat:(\d.\d\d)_dcha:(\d.\d\d)_dfin:(\d.\d\d)/opt_log"
groups = re.match(reg,opt_log).groups()
print(groups[0] + f"/preprocessed/realcase/vehicles.base.dbat:{groups[2]}_dcha:{groups[3]}_dfin:{groups[4]}.{groups[1]}.csv.gz")


