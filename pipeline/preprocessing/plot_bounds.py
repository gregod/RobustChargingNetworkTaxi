import numpy as np
import matplotlib.pyplot as plt
import tikzplotlib
import sys
import re

_,logpath,pngpath,texpath,pngpathtime,texpathtime = sys.argv

file1 = open(logpath, 'r')
count = 0
lb=[]
ub=[]
timestamps=[]
markers=[]
while True:
    line = file1.readline()
    if not line:
        break
    if line.startswith("SOLVED FIRST LEVEL"):
        markers.append((len(lb),timestamps[-1],"Second Level"))
    if line.startswith("Best first level cost"):
        val=int(re.search(r"cost (\d*) with",line).group(1))
        lb.append(val)
        ub.append(val)
    if line.startswith("Best cost"):
        val=int(re.search(r"cost (\d*) with",line).group(1))
        lb.append(val)
        ub.append(val)
        timestamps.append(int(timestamps[-1]))
    if not line.startswith("BEND"):
        continue
    count += 1
    el = line.split("|")
    lb.append(int(el[2]))
    ub.append(int(el[3]))
    timestamps.append(int(el[4]))
file1.close()
x = list(range(0,len(lb)))


c_ub = [np.nan if i > 1e5 else i for i in ub]

# with x = iterations
plt.figure(figsize=(15,5))
for (mark,timestamp, label) in markers:
    plt.axvline( mark , color='gray',ls="--" )
    plt.text(mark, 20, label, va='center', ha='center', backgroundcolor='w')

plt.plot(x,lb,label="Lower Bound")
plt.plot(x,c_ub,label="Upper Bound")
plt.legend(loc="upper right")
plt.xlabel("Iterations")
plt.ylabel("Cost")
tikzplotlib.save(texpath)
plt.savefig(pngpath)

# with x = timestamps
plt.figure(figsize=(15,5))
for (mark, timestamp,label) in markers:
    plt.axvline( timestamp , color='gray',ls="--" )
    plt.text(timestamp, 20, label, va='center', ha='center', backgroundcolor='w')

plt.plot(timestamps,lb,label="Lower Bound")
plt.plot(timestamps,c_ub,label="Upper Bound")
plt.legend(loc="upper right")
plt.xlabel("Duration (s)")
plt.ylabel("Cost")
tikzplotlib.save(texpathtime)
plt.savefig(pngpathtime)