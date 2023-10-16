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
        help = "Site file, best to use full sites"
        arg_type = String
        required = true
    "--trips"
        help = "Trips file, best to use full sites"
        arg_type = String
        required = true

    "--output"
        help = "Output filename"
        arg_type = String
        required = true
end
args = parse_args(s)


site_file = args["sites"]
trips_file = args["trips"]
output_filename = args["output"]

min_per_period = 5
dfs = DataFrame(CSV.File(site_file,ignoreemptylines = true));
dft = DataFrame(CSV.File(transcode(GzipDecompressor, Mmap.mmap(trips_file)),ignoreemptylines = true));


dfs.location =  map((v) -> SVector(parse.(Float32,v)...),split.(chop.(dfs.location,head=1,tail=1),","));

dft.startPoint =  map((v) -> SVector(parse.(Float32,v)...),split.(chop.(dft.startPoint,head=1,tail=1),","));
dft.endPoint =  map((v) -> SVector(parse.(Float32,v)...),split.(chop.(dft.endPoint,head=1,tail=1),","));
dft.start_lat = first.(dft.startPoint)
dft.start_lon = last.(dft.startPoint)
dft.end_lat = first.(dft.endPoint)
dft.end_lon = last.(dft.endPoint)

function get_distance(starts, ende, numtry = 0)
    startsstr = join(map((x) -> "$(x[2]),$(x[1])",starts),";")
    endesstr = join(map((x) -> "$(x[2]),$(x[1])",ende),";")
    sourcesstr=join(range(0,size(starts,1)-1,step=1),";")
    destinationstr=join(range(size(starts,1),length=size(ende,1)),";")

    url = "http://127.0.0.1:5000/table/v1/driving/$(startsstr);$(endesstr)?sources=$(sourcesstr)&destinations=$(destinationstr)&annotations=distance,duration&skip_waypoints=true"
    
    try
        return JSON.parse(String(HTTP.get(url, retries=3).body))
    catch e
        if numtry < 5
            return get_distance(starts,ende, numtry + 1)
        else
            throw(e)
        end
    end
end

function get_direct_distance(start,ende)
    url = "http://127.0.0.1:5000/route/v1/driving/$(start[2]),$(start[1]);$(ende[2]),$(ende[1])"
    try
        resp = JSON.parse(String(HTTP.get(url, retries=3).body))
        data = resp["routes"][1]
        if data["distance"] <= 0
            throw(DomainError)
        end
        data
    catch
        Dict(
            "distance" => haversine(start,ende,6372.8) * 1000,
            "duration" => haversine(start,ende,6372.8) * 0.001414
        )
    end
end



local_lookup = [ Dict{NTuple{4,Float32},NTuple{2,Float32}}() for i in 1:Threads.nthreads()]

trips = collect(zip(dft.startPoint,dft.endPoint))
p = ProgressMeter.Progress(length(trips), desc="Computing Trip Distance")
Threads.@threads for (start,ende) in trips
    datas = get_direct_distance(start,ende)
    local_lookup[Threads.threadid()][(start[1],start[2],ende[1],ende[2])] = (datas["duration"],datas["distance"])
    ProgressMeter.next!(p)
end

start_points = collect(Iterators.partition(dft.startPoint,500))
p = ProgressMeter.Progress(length(start_points), desc="Computing To Distance")
Threads.@threads for t in start_points
    datas = get_distance(t,dfs.location)
    for (start,distances,durations) in zip(t,datas["distances"],datas["durations"])
        for (loc,dist,dur) in zip(dfs.location,distances,durations)
            @assert(dist < 100000)
            if dist <= 0
                direct_dist = haversine(start,loc,6372.8)
                @assert(0 <= direct_dist < 100000, "Invaliud Direct Dist $direct_dist")
                local_lookup[Threads.threadid()][(start[1],start[2],loc[1],loc[2])] = (direct_dist* 0.001414 + 1 ,direct_dist* 1000 + 1)
            else
                local_lookup[Threads.threadid()][(start[1],start[2],loc[1],loc[2])] = (dur,dist)
            end

        end
    end
    ProgressMeter.next!(p)
end



end_points = collect(Iterators.partition(dft.endPoint,500))
p = ProgressMeter.Progress(length(end_points), desc="Computing From Distance")
Threads.@threads for t in end_points
    datas = get_distance(dfs.location,t)
    for (start,distances,durations) in zip(dfs.location,datas["distances"],datas["durations"])
        for (loc,dist,dur) in zip(t,distances,durations)
            @assert(dist < 100000)
            if dist <= 0
                direct_dist = haversine(start,loc,6372.8)
                @assert(direct_dist < 100000)
                local_lookup[Threads.threadid()][(start[1],start[2],loc[1],loc[2])] = (direct_dist* 0.001414,direct_dist* 1000)
            else
                local_lookup[Threads.threadid()][(start[1],start[2],loc[1],loc[2])] = (dur,dist)
            end
        end
    end
    ProgressMeter.next!(p)
end

function do_it(local_lookup,output_filename)
	lookup::Dict{NTuple{4,Float32},NTuple{2,Float32}} = merge(local_lookup...)
	open(f -> serialize(f,lookup), output_filename, "w");
end

do_it(local_lookup,output_filename)
