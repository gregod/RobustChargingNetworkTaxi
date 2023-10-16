"""
Takes a trip and site file and precomputes the list of
reachable sites for every trip.

Arguments: site_file, trips_file, min_per_meter
"""

min_per_period = 5

import pandas as pd

import sys

from math import radians, cos, sin, asin, sqrt
import io



output = io.StringIO()

_, site_file, trips_file, min_per_meter = sys.argv



min_per_meter = float(min_per_meter)
df_site = pd.read_csv(site_file)

df_site.index = df_site["id"]
df_site["location"] = list(df_site["location"].map(lambda x: list(map(float, x[1:-1].split(", ")))))
df_site["lat"] = list(df_site["location"].map(lambda x: x[0]))
df_site["lon"] = list(df_site["location"].map(lambda x: x[1]))
df_site["idx"] = range(len(df_site))


df_trips =  pd.read_csv(trips_file)

df_trips["startPoint"] = list(df_trips["startPoint"].map(lambda x: list(map(float, x[1:-1].split(", ")))))
df_trips["start_lat"] = list(df_trips["startPoint"].map(lambda x: x[0]))
df_trips["start_lon"] = list(df_trips["startPoint"].map(lambda x: x[1]))

df_trips["endPoint"] = list(df_trips["endPoint"].map(lambda x: list(map(float, x[1:-1].split(", ")))))
df_trips["end_lat"] = list(df_trips["endPoint"].map(lambda x: x[0]))
df_trips["end_lon"] = list(df_trips["endPoint"].map(lambda x: x[1]))



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


def get_distance_air(lat1,lon1,lat2,lon2):
    distance_in_meters = int(round(haversine( (lat1, lon1), (lat2, lon2))))
    distance_in_minutes = distance_in_meters * min_per_meter
    return (distance_in_meters,distance_in_minutes)

import requests

def get_distance_osmr(lat1,lon1,lat2,lon2):
    URL = "http://127.0.0.1:5000/route/v1/driving/%f,%f;%f,%f"

    r = requests.get(url =  URL % (lon1,lat1,lon2,lat2))
    json = r.json()
    if "routes" not in json:
        # if there is no route this is usually as points a very close
        # use air distance for these cases
        air_distances = get_distance_air(lat1,lon1,lat2,lon2)
        if air_distances[0] > 500:
            print("No routes found for ",lat1,lon1,"to",lat2,lon2, file=sys.stderr)
            print("But air distance >500m (=",air_distances[0],")",file=sys.stderr)
            exit(1)
        return air_distances

    data = json["routes"][0]

    return (int(round(data["distance"])),data["duration"] / 60)

def get_distances_bulk_osmr(start_lat,start_lon,finish_lat,finish_lon):
    trip_str  = "%f,%f;%f,%f" % (start_lon,start_lat,finish_lon,finish_lat);
    str_location_sites = df_site["lon"].map(str) + "," + df_site["lat"].map(str)
    site_str = ";".join(str_location_sites)
    URL = ("http://127.0.0.1:5000/table/v1/driving/%s;%s?annotations=distance,duration&skip_waypoints=true" % (trip_str,site_str))
    r = requests.get(url = URL)
    data = r.json()
    return (data["distances"],data["durations"])


def get_distance_trip2site(bulk_data,site_id):
    distance_m = int(round(bulk_data[0][0][df_site.loc[site_id]["idx"]+2]))
    duration_m  = bulk_data[1][0][df_site.loc[site_id]["idx"]+2]/60
    if distance_m < 0:
        print("DISTANCE is ",distance_m,file=sys.stderr)
        distance_m = 0
        duration_m = 0
    if distance_m < -200:
        print("DISTANCE is ",distance_m,"exiting!",file=sys.stderr)
        exit(1)
    return distance_m,duration_m

def get_distance_site2trip(bulk_data,site_id):
    distance_m = int(round(bulk_data[0][df_site.loc[site_id]["idx"]+2][1]))
    duration_m = bulk_data[1][df_site.loc[site_id]["idx"]+2][1]/60
    if distance_m < 0:
        print("DISTANCE is ",distance_m,file=sys.stderr)
        distance_m = 0
        duration_m = 0
    if distance_m < -200:
        print("DISTANCE is ",distance_m,"exiting!",file=sys.stderr)
        exit(1)
    return (distance_m,duration_m)





def find_possible_sites(trip_row):

    if not trip_row["isFree"]:
        return ""


    bulk_distances = get_distances_bulk_osmr(trip_row["start_lat"],trip_row["start_lon"],trip_row["end_lat"],trip_row["end_lon"])
    local_sites = df_site.copy()



    distance_tuples_to = local_sites.apply(lambda site_row: (
        get_distance_trip2site(bulk_distances,site_row["id"])
    ), axis=1)



    local_sites["distance_driving_to"] = list(map(lambda x: x[0],distance_tuples_to))
    local_sites["driving_to_in_periods"] = list(map(lambda x: int(round(x[1] / min_per_period)),distance_tuples_to))

    distance_tuples_from = local_sites.apply(lambda site_row: (
            (
                get_distance_site2trip(bulk_distances,site_row["id"])
            )), axis=1)


    local_sites["distance_driving_from"] = list(map(lambda x: x[0],distance_tuples_from))
    local_sites["driving_from_in_periods"] = list(map(lambda x: int(round(x[1] / min_per_period)),distance_tuples_from))


    free_time_in_periods = (trip_row["endPeriod"] - trip_row["startPeriod"]) + 1


    local_sites["site_arrival"] = trip_row["startPeriod"] + local_sites["driving_to_in_periods"]
    local_sites["site_departure"] = trip_row["endPeriod"] - local_sites["driving_from_in_periods"]

    theoretical_sites = local_sites[
        (free_time_in_periods - local_sites["driving_to_in_periods"] - local_sites["driving_from_in_periods"] > 0)
        & (local_sites["driving_to_in_periods"] < 60000)
        & (local_sites["driving_from_in_periods"] < 60000)
        ].sort_values(by=["distance_driving_to"])
    

    if len(theoretical_sites) == 0:
        return ""

    return  ";".join(theoretical_sites.apply(lambda el : "%s[%d|%d|%d|%d]" % (el["id"],el["site_arrival"],el["distance_driving_to"],el["site_departure"],el["distance_driving_from"]),axis=1))



def find_real_distance_of_trip(trip_row):
    osm_distance_tuple = get_distance_osmr(trip_row["start_lat"],trip_row["start_lon"],trip_row["end_lat"],trip_row["end_lon"])
    dist = int(osm_distance_tuple[0])
    if dist < 0:
        print(trip_row, file=sys.stderr)
    return dist



df_trips["potentialSites"] = df_trips.apply(find_possible_sites, axis=1)
df_trips["osmDistance"] = df_trips.apply(find_real_distance_of_trip,axis=1)





df_trips.to_csv(output,index=False)
print(output.getvalue())
