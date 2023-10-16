
# %%
import pandas as pd
import numpy as np
from scipy import stats
from sklearn.neighbors import KernelDensity
import random
import io
import sys
from math import radians, cos, sin, asin, sqrt
#%%
_, site_input, trip_input, num_sites = sys.argv

#trip_input = "/home/gregor/Code/et/pipeline/input_data/trips.csv.gz"
#site_input = "/home/gregor/Code/et/pipeline/work/preprocessed/nocost_f215.sites.csv"
#num_sites = 60

# %%
np.random.seed(1234)



df = pd.read_csv(trip_input,usecols=["startPoint","isFree"])
dfs = pd.read_csv(site_input)
num_sites = int(num_sites)
output = io.StringIO()
# %%

free_trips = df[df["isFree"]]

free_trips["start_point"] = list(free_trips["startPoint"].map(lambda x: list(map(float, x[1:-1].split(", ")))))


startX = list(map(lambda x: x[0],free_trips["start_point"]))
startY = list(map(lambda x: x[1],free_trips["start_point"]))

xy_train  = np.vstack([startY, startX]).T
kde_skl = KernelDensity(bandwidth = 0.002)
kde_skl.fit(xy_train)
points = kde_skl.sample(num_sites*100)
# %%
dfs["location_arr"] = list(dfs["location"].map(lambda x: list(map(float, x[1:-1].split(", ")))))
dfs["lat"] = list(dfs["location_arr"].map(lambda x: x[0]))
dfs["lon"] = list(dfs["location_arr"].map(lambda x: x[1]))
# %%

# mean earth radius - https://en.wikipedia.org/wiki/Earth_radius#Mean_radius
AVG_EARTH_RADIUS_KM = 6371.0088

def haversine(point1, point2):
    """ Calculate the great-circle distance between two points on the Earth surface.
    :input: two 2-tuples, containing the latitude and longitude of each point
    in decimal degrees.
    Example: haversine((45.7597, 4.8422), (48.8567, 2.3508))
    :output: Returns the distance between the two points in meters
    """
    # get earth radius in required units
    avg_earth_radius = AVG_EARTH_RADIUS_KM

    # unpack latitude/longitude
    lat1, lng1 = point1
    lat2, lng2 = point2

    # convert all latitudes/longitudes from decimal degrees to radians
    lat1, lng1, lat2, lng2 = map(radians, (lat1, lng1, lat2, lng2))

    # calculate haversine
    lat = lat2 - lat1
    lng = lng2 - lng1
    d = sin(lat * 0.5) ** 2 + cos(lat1) * cos(lat2) * sin(lng * 0.5) ** 2
    distance_m = 2 * avg_earth_radius * asin(sqrt(d)) * 1000
    return distance_m

selected_sites = []
for p in points:
    if len(selected_sites) == num_sites:
        break

    local_sites = dfs[["location_arr","id"]]
    local_sites["distance"] = list(local_sites["location_arr"].map(lambda site: haversine((site[1],site[0]),p) ))
    
    for ix,potential_site in local_sites.sort_values(by="distance").head(5).iterrows():
        if potential_site["distance"] > 2000:
            break
        if potential_site["id"] not in selected_sites:
            selected_sites.append(potential_site["id"])
            break



# %%

# %%
dfs.index = dfs["id"]
selected_sites = dfs.loc[selected_sites]
# %%

selected_sites.drop(["location_arr"],axis=1).to_csv(output,index=False)
print(output.getvalue())
# %%
