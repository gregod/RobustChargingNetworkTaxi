# the site steps are further used within the clustering scheme to cluster downwards from the max
site_steps = [20,30,40,50,60,70,80,90,100,110,120,130,140,150]
wildcard_constraints:
    INT_NUM_SITES="\d*", #  int number for site
    NUM_SITES="\d*(|_\w*)", #  number for site
    SUFFIX_DASH="(|_\w*)",
    time_bucket="(\d\d\d\d-\d\d-\d\d-\d\d:\d\d:\d\d)|(virtual_\d*-\d*)", # either date string or virtual_{dow}-{length}
    is_var="|var_", # can be empty or "var_" prefix
    TOLERANCE="\d\d",
    TYPE_GROUP="\d*",
    NUM_VEHICLES="\d*",
    cross_or_opt="(cross)|(opt)",
    is_cross="(|(cross_))",
    is_leveled="(|(leveled_))",
    SEED="\d*",
    SEED_TYPE="(median)|(lowest)",
    BATTERY="(battery_\d)|(dbat:\d*.\d\d_dcha:\d*.\d\d_dfin:\d*.\d\d)",
    QUORUM_ACCEPT="\d*",
    BENEVOLENT="\d*",
    DOT_LEVELED="(|(\.leveled))"

# find the fixed step one bigger (or all at the biggest)
# used to check if im worse than better
def find_taxi_step(wildcards):
    num_sites = int(wildcards.NUM_SITES)
    if num_sites >= max(site_steps):
        return "input_data/taxi_sites.csv"
    else:
        larger_step = site_steps[site_steps.index(num_sites)+1]
        return OUTPUT_PREFIX + "/preprocessed/nocost_{}.sites.csv".format(larger_step)



def get_bigger_step(wildcards):
    num_sites = int(wildcards.NUM_SITES)
    if num_sites >= max(site_steps):
        return max(site_steps)
    else:
        larger_step = site_steps[site_steps.index(num_sites)+1]
        return larger_step

# spawn bucket target for every time bucket generated
def generate_other_trips_input(wildcards):

    # detect if we are virtual or not
    if wildcards.time_bucket.startswith("virtual"):
        virtual_windowlen = wildcards.time_bucket.split("-")[1]
        return expand(
            os.path.join(OUTPUT_PREFIX,"preprocessed","{{NUM_SITES}}","virtual_{time_bucket}-" + virtual_windowlen,"{{BATTERY}}","tol{{TOLERANCE}}" ,"final.trips.csv"),
            time_bucket=[ i for i in days_of_week if "virtual_" + str(i) + "-" + virtual_windowlen != wildcards.time_bucket]
        )

    else:
        return expand(
            os.path.join(OUTPUT_PREFIX,"preprocessed","{{NUM_SITES}}","{time_bucket}","{{BATTERY}}","tol{{TOLERANCE}}","final.trips.csv"),
            time_bucket=[ i for i in TIME_BUCKETS if i != wildcards.time_bucket]
        )

# spawn bucket target for every time bucket generated
def generate_other_vehicles_input(wildcards):
        # detect if we are virtual or not
        if wildcards.time_bucket.startswith("virtual"):
            virtual_windowlen = wildcards.time_bucket.split("-")[1]
            return expand(
                os.path.join(OUTPUT_PREFIX,"preprocessed","{{NUM_SITES}}","virtual_{time_bucket}-" + virtual_windowlen ,"{{BATTERY}}","tol{{TOLERANCE}}","final.vehicles.csv"),
                time_bucket=[ i for i in days_of_week if "virtual_" + str(i) + "-" + virtual_windowlen != wildcards.time_bucket]
            )

        else:
            return expand(
                os.path.join(OUTPUT_PREFIX,"preprocessed","{{NUM_SITES}}","{time_bucket}","{{BATTERY}}","tol{{TOLERANCE}}","final.vehicles.csv"),
                time_bucket=[ i for i in TIME_BUCKETS if i != wildcards.time_bucket]
            )

def generate_other_vehicles_input_nonleveled(wildcards):

    # detect if we are virtual or not
    if wildcards.time_bucket.startswith("virtual"):
        virtual_windowlen = wildcards.time_bucket.split("-")[1]
        return expand(
            os.path.join(OUTPUT_PREFIX,"preprocessed","{{NUM_SITES}}","virtual_{time_bucket}-" + virtual_windowlen ,"{{BATTERY}}","tol{{TOLERANCE}}","final.vehicles.csv"),
            time_bucket=[ i for i in days_of_week if "virtual_" + str(i) + "-" + virtual_windowlen != wildcards.time_bucket]
        )

    else:
        return expand(
            os.path.join(OUTPUT_PREFIX,"preprocessed","{{NUM_SITES}}","{time_bucket}","{{BATTERY}}","tol{{TOLERANCE}}","final.vehicles.csv"),
            time_bucket=[ i for i in TIME_BUCKETS if i != wildcards.time_bucket]
        )
