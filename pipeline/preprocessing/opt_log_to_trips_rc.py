import re
import sys
"""
Find the matching trip input file from the opt log
"""
import re
opt_log = sys.argv[1]
#opt_log = "/work/opt/realcase/1/dbat:1.00_dcha:1.00_dfin:1.00/opt_log"
# "/opt/realcase/{DAY}/dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}/opt_log"
# >> "/preprocessed/realcase/trips.all.{DAY}.csv.gz",

reg = "(.*)/opt/realcase/(\d*)/dbat:(\d.\d\d)_dcha:(\d.\d\d)_dfin:(\d.\d\d)/opt_log"
groups = re.match(reg,opt_log).groups()
print(groups[0] + f"/preprocessed/realcase/trips.all.{groups[1]}.csv.gz")