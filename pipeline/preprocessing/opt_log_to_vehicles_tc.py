import re
import sys

"""
Find the matching vehicle input file from the opt log
"""
import re
opt_log = sys.argv[1]
#opt_log = "/work/opt/realcase/1/dbat:1.00_dcha:1.00_dfin:1.00/opt_log"
# >> vehicles=OUTPUT_PREFIX + "/preprocessed/realcase/vehicles.base.dbat:{DBAT}_dcha:{DCHAR}_dfin:{DFINAL}.{DAY}.csv.gz",

# "/opt/techcase/{SEED}/group_{TYPE_GROUP}/{BATTERY}/tol{TOLERANCE}/{INT_NUM_SITES}{SUFFIX_DASH}/{NUM_VEHICLES}/opt_log"
# OUTPUT_PREFIX + "/preprocessed/techcase/{SEED}/group_{TYPE_GROUP}/{INT_NUM_SITES}/{NUM_VEHICLES}/{BATTERY}.final.vehicles.csv.gz",
reg = "(.*)/opt/techcase/(\d*)/group_(\d*)/dbat:(\d.\d\d)_dcha:(\d.\d\d)_dfin:(\d.\d\d)/tol(\d*)/(\d*)/(\d*)/opt_log"
groups = re.match(reg,opt_log).groups()
print(groups[0] + f"/preprocessed/techcase/{groups[1]}/group_{groups[2]}/{groups[7]}/{groups[8]}/dbat:{groups[3]}_dcha:{groups[4]}_dfin:{groups[5]}.final.vehicles.csv.gz")


