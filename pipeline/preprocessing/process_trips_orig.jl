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
    "--output"
        help = "Output"
        arg_type = String
        nargs = '+'
        required = true
end
args = parse_args(s)



site_file = args["sites"]
trips_files = args["trips"]
outputs = args["output"]


function parseLocation(input::String)::SVector{2,Float64}
    return map((v) -> parse(Float64, v), split(chop(input, head=1, tail=1), ","))
end


struct ReachableSite 
    site_id::String
    site_arrival::Int64
    driving_to_in_periods::Int64
    distance_driving_to::Float64
    site_departure::Int64
    distance_driving_from::Float64
    driving_from_in_periods::Int64
end



function copy_trips_to_out(site_file::String,trips_file::String,out_file_name::String)
    


    
    dfs = DataFrame(CSV.File(site_file, ignoreemptyrows=true,stringtype=String));
    site_ids = dfs.id


    out_file = GZip.open(out_file_name,"w")
    do_append = false
    row_counter = 0
    for f in CSV.Rows(transcode(GzipDecompressor, Mmap.mmap(trips_file)), ignoreemptyrows=true, reusebuffer=true, stringtype=String)

        replacementPotentialSites = ""
        if f.potentialSites !== missing

            potentialSites = split(f.potentialSites,";")
            potential_ids = (map(x -> in(split(x,"[")[1],site_ids),potentialSites))
            replacementPotentialSites =join( Iterators.take(potentialSites[potential_ids],15),";")
        end

        keys = tuple(propertynames(f)...)
        val = [values(f)...]

        potentialSitesIdx = findfirst(isequal(:potentialSites),keys)
        val[potentialSitesIdx] = replacementPotentialSites

        CSV.write(out_file,[NamedTuple{keys}(val)],append=do_append)

        do_append = true
    
    end
    close(out_file)
end


for (trip_file,output_file) in zip(trips_files,outputs)
    copy_trips_to_out(site_file,trip_file,output_file)
end

