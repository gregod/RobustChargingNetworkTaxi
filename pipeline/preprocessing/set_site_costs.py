"""
Takes a list of sites and updates their costs based on a distance formula
"""
import csv
import numpy as np
import pandas as pd
import sys
from math import radians, cos, sin, asin, sqrt,exp
import io


import random

output = io.StringIO()

_, input_file = sys.argv

df = pd.read_csv(input_file)
df.index = df["id"]
df["location"] = list(df["location"].map(lambda x: list(map(float, x[1:-1].split(", ")))))

df["lat"] = list(df["location"].map(lambda x: x[1]))
df["lon"] = list(df["location"].map(lambda x: x[0]))

munich_center = (11.5759,48.1381)


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



def easing_cost(distance_km):
    relative_distance = distance_km / 32
    relative_cost =  30 * exp(-10*relative_distance) - relative_distance * 10
    return int(20 + relative_cost )


df["cost"] = df.apply(lambda row: easing_cost(haversine(munich_center,(row["lat"],row["lon"]))/1000), axis=1)


csvwriter = csv.writer(output, delimiter=',',
                        quotechar='"', quoting=csv.QUOTE_NONNUMERIC
                        )
csvwriter.writerow(['id', 'capacity', 'cost', 'location'])

for index,row in df.iterrows():
    csvwriter.writerow([row["id"],row["capacity"],row["cost"],row["location"]])

print(output.getvalue())

