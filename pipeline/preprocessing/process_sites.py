"""
Takes a list of sites and clusters the result down
to given n using kmeans. Sites closest to cluster centers
are selected as cluster representative

Arguments: input_file n_clusters
"""
import csv
import numpy as np
import pandas as pd
import sys
from math import radians, cos, sin, asin, sqrt
from shapely.geometry import MultiPoint
from sklearn.cluster import KMeans
import io


import random

output = io.StringIO()

_, input_file, n_clusters = sys.argv
n_clusters = int(n_clusters)

df = pd.read_csv(input_file)
df.index = df["id"]
df["location"] = list(df["location"].map(lambda x: list(map(float, x[1:-1].split(", ")))))

df["lat"] = list(df["location"].map(lambda x: x[1]))
df["lon"] = list(df["location"].map(lambda x: x[0]))

munich_center = (11.5759,48.1381)

# site clustering aproach
coords = df[['lat', 'lon']].values
c_algo = KMeans(n_clusters=n_clusters)
db = c_algo.fit(np.radians(coords))

cluster_labels = db.labels_

num_clusters = len(set(cluster_labels))
clusters = pd.Series([coords[cluster_labels == n] for n in range(num_clusters)])



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


def get_centermost_point(cluster):
    centroid = (MultiPoint(cluster).centroid.x, MultiPoint(cluster).centroid.y)
    centermost_point = min(cluster, key=lambda point: haversine(point, centroid))
    return tuple(centermost_point)


centermost_points = clusters.map(get_centermost_point)

fun = lambda x: centermost_points[x]

df["new_location"] = list(map(fun, db.labels_))
df["include"] = df.apply(lambda row: row["new_location"] == (row["lat"],row["lon"]),axis=1)

df = df.sort_values(by=['cost'],ascending=True)

spamwriter = csv.writer(output, delimiter=',',
                        quotechar='"', quoting=csv.QUOTE_NONNUMERIC
                        )
spamwriter.writerow(['id', 'capacity', 'cost', 'location'])

for index,row in df[df["include"]].head(n_clusters).iterrows():
    spamwriter.writerow([row["id"],row["capacity"],row["cost"],row["location"]])

print(output.getvalue())

