using CSV,CodecZlib, Mmap
using DataFrames
using DataFramesMeta
using Dates
using StaticArrays
using Distances
using ProgressMeter
using GZip
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
        nargs = '+'
        arg_type = String
        required = true
    "--distance-cache"
        help = "Distance Matrix cache file"
        arg_type = String
        required = true
    "--output"
        help = "Output"
        arg_type = String
        nargs = '+'
        required = true
end
args = parse_args(s)



site_file = args["sites"]
trips_files = args["trips"]
cache_file = args["distance-cache"]
outputs = args["output"]


function parseLocation(input::String)::SVector{2,Float32}
    return map((v) -> parse(Float32, v), split(chop(input, head=1, tail=1), ","))
end


struct ReachableSite 
    site_id::String
    site_arrival::Int64
    driving_to_in_periods::Int64
    distance_driving_to::Float32
    site_departure::Int64
    distance_driving_from::Float32
    driving_from_in_periods::Int64
end

const min_per_period = 5

function process_site(lookup::Dict{NTuple{4,Float32},NTuple{2,Float32}}, site_id::String, site_location::SVector{2,Float32}, startPoint::SVector{2,Float32}, endPoint::SVector{2,Float32}, startPeriod::Int, endPeriod::Int)
    
    
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



function copy_trips_to_out(lookup::Dict{NTuple{4,Float32},NTuple{2,Float32}},site_file::String,trips_file::String,out_file_name::String,cache_file::String)
    


    
    dfs = DataFrame(CSV.File(site_file, ignoreemptylines=true));
    dfs.point_location =  parseLocation.(dfs.location)


    filter_func = free_time -> d::ReachableSite -> begin 
        time_cirt =  free_time - d.driving_to_in_periods - d.driving_from_in_periods > 0
        distance_crit = d.driving_to_in_periods < 60000 && d.driving_from_in_periods < 60000 # u16 int 
        time_cirt && distance_crit
    end

    sort_func =  v -> v.distance_driving_to
    map_func = r -> @sprintf "%s[%d|%d|%d|%d]" r.site_id r.site_arrival r.distance_driving_to r.site_departure r.distance_driving_from
    take_func = Iterators.take


    write(stderr, "Loaded distance cache\n");
    out_file = GZip.open(out_file_name,"w")
    do_append = false
    row_counter = 0
    write(stderr, "Starting row loop\n");
    for f in CSV.Rows(transcode(GzipDecompressor, Mmap.mmap(trips_file)), ignoreemptylines=true, reusebuffer=true)

        row_counter += 1

        if mod(row_counter,10000) == 0
            write(stderr, "Flushing row loop\n");
            flush(out_file)
        end
        startPoint =  parseLocation(f.startPoint);
        endPoint =  parseLocation(f.endPoint);

        startPeriod = parse(Int,f.startPeriod)
        endPeriod = parse(Int,f.endPeriod)

        start_lat::Float32 = first(startPoint)
        start_lon::Float32 = last(startPoint)
        end_lat::Float32 = first(endPoint)
        end_lon::Float32 = last(endPoint)


        free_time = endPeriod - startPeriod + 1
    


        dist = round(lookup[(startPoint[1],startPoint[2],endPoint[1],endPoint[2])][2])

        @assert(dist <= 100000)
        osmDistance = dist

        reachable_sites = filter(
            filter_func(free_time),
            process_site.(Ref(lookup),dfs["id"], dfs["point_location"],Ref(startPoint),Ref(endPoint),Ref(startPeriod),Ref(endPeriod))
        )

        sort!(reachable_sites, by=sort_func)

        ret = map(map_func , reachable_sites)
        potentialSites =  join(ret, ";")
        
        keys = tuple(propertynames(f)...,:start_lat,:start_lon,:end_lat,:end_lon,:potentialSites,:osmDistance)
        val = [values(f)...,start_lat,start_lon,end_lat,end_lon,potentialSites,osmDistance]


        CSV.write(out_file,[NamedTuple{keys}(val)],append=do_append)

        do_append = true
        

    end
    write(stderr, "Completed row loop\n");
    close(out_file)
end

const lookup  = open(deserialize,cache_file) 

for (trip_file,output_file) in zip(trips_files,outputs)
    copy_trips_to_out(lookup,site_file,trip_file,output_file,cache_file)
end

