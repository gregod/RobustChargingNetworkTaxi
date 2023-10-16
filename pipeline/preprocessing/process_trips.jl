using CSV,CodecZlib, Mmap
using DataFrames
using DataFramesMeta
using Dates
using StaticArrays
using Distances
using ProgressMeter
using JSON
using HTTP
using Printf
using Serialization

using ArgParse


s = ArgParseSettings()
@add_arg_table s begin
    "--sites"
        help = "Site file"
        arg_type = String
        required = true
    "--trips"
        help = "Trips file"
        arg_type = String
        required = true

    "--distance-cache"
        help = "Distance Matrix cache file"
        arg_type = String
        required = true
    
    "--output"
        help = "Output"
        arg_type = String
        required = false
        default = "/dev/stdout"
end
args = parse_args(s)




site_file = args["sites"]
trips_file = args["trips"]
cache_file = args["distance-cache"]
output = args["output"]

min_per_period = 5
dfs = DataFrame(CSV.File(site_file, ignoreemptylines=true));
dft = DataFrame(CSV.File(transcode(GzipDecompressor, Mmap.mmap(trips_file)), ignoreemptylines=true));


dfs.location =  map((v) -> SVector(parse.(Float64, v)...), split.(chop.(dfs.location, head=1, tail=1), ","));

dft.startPoint =  map((v) -> SVector(parse.(Float64, v)...), split.(chop.(dft.startPoint, head=1, tail=1), ","));
dft.endPoint =  map((v) -> SVector(parse.(Float64, v)...), split.(chop.(dft.endPoint, head=1, tail=1), ","));
dft.start_lat = first.(dft.startPoint)
dft.start_lon = last.(dft.startPoint)
dft.end_lat = first.(dft.endPoint)
dft.end_lon = last.(dft.endPoint)





        
lookup  = open(deserialize,cache_file)

struct ReachableSite 
    site_id::String
    site_arrival::Int64
    driving_to_in_periods::Int64
    distance_driving_to::Float64
    site_departure::Int64
    distance_driving_from::Float64
    driving_from_in_periods::Int64
end

process_site = (site_id, site_location, startPoint, endPoint, startPeriod, endPeriod) -> begin
    lookTo = lookup[(startPoint[1],startPoint[2],site_location[1],site_location[2])]
    lookFrom = lookup[(site_location[1],site_location[2],endPoint[1],endPoint[2])]
    distance_driving_to = lookTo[2]
    driving_to_in_periods = Integer(round(lookTo[1] / 60 / min_per_period))
    distance_driving_from = lookFrom[2]
    driving_from_in_periods = Integer(round(lookFrom[1] / 60 / min_per_period))

    @assert distance_driving_to >= 0
    @assert distance_driving_from >= 0

    @assert driving_to_in_periods >= 0
    @assert driving_from_in_periods >= 0

    @assert startPeriod >= 0
    @assert endPeriod >= 0

    ReachableSite(
        site_id,
        startPeriod + driving_to_in_periods,
        driving_to_in_periods,
        distance_driving_to,
        endPeriod - driving_from_in_periods,
        distance_driving_from,
        driving_from_in_periods
    )   
end


filter_func = free_time -> d -> begin 
    time_cirt =  free_time - d.driving_to_in_periods - d.driving_from_in_periods > 0
    distance_crit = d.driving_to_in_periods < 60000 && d.driving_from_in_periods < 60000 # u16 int 
    time_cirt && distance_crit
end
sort_func =  v -> v.distance_driving_to
map_func = r -> @sprintf "%s[%d|%d|%d|%d]" r.site_id r.site_arrival r.distance_driving_to r.site_departure r.distance_driving_from

const take_func = Iterators.take

out_dft = @byrow!(dft, begin
    @newcol  potentialSites::Array{String};
    @newcol  osmDistance::Array{Float64};
    
    free_time = :endPeriod - :startPeriod + 1


    (dur,dist) = round.(lookup[(:startPoint[1],:startPoint[2],:endPoint[1],:endPoint[2])][2])

    @assert(dist <= 100000)
    :osmDistance = dist

    reachable_sites = filter(
        filter_func(free_time),
        process_site.(dfs["id"], dfs["location"],Ref(:startPoint),Ref(:endPoint),Ref(:startPeriod),Ref(:endPeriod))
    )

    sort!(reachable_sites, by=sort_func)

    ret = map(map_func , reachable_sites)
    :potentialSites =  join(ret, ";")
end
)

CSV.write(output, out_dft)
