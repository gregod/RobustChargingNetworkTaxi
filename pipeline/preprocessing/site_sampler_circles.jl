using CSV,CodecZlib, Mmap
using DataFrames
using DataFramesMeta
using Dates
using Clustering
using Distances
using Statistics
using Random

using ArgParse



# for repl

if false 
    sites = "input_data/taxi_sites.csv"
    split_distance = 3
    inner_count = 15
    outer_count = 45
end

s = ArgParseSettings()
@add_arg_table s begin
    "--sites"
        help = "Input site file"
        arg_type = String
        required = true
    "--inner-circle-size"
        help = "Size of inner circle in km"
        arg_type = Float16
        required = true

    "--inner-count"
        help = "Number of sites in inner cirlce"
        arg_type = Int
        required = true
    
    "--outer-count"
        help = "Number of sites in outer circle"
        arg_type = Int
        required = true
end
args = parse_args(s)
sites = args["sites"]
split_distance = args["inner-circle-size"]
inner_count = args["inner-count"]
outer_count = args["outer-count"]



cluster_kmedoids = (locations,count=30) -> begin
    distances = pairwise(Euclidean(), locations)
    kmedoids(distances,count,init=:kmpp).medoids
end

cluster_kmedian = (locations,count=30) -> begin
    kmean_centers = kmeans(locations,count).centers
    closest_points = argmin(pairwise(Euclidean(), kmean_centers,locations),dims=2)
    (x -> x[2]).(closest_points)
end


Random.seed!(1234)

dfs = DataFrame(CSV.File(sites));
dfs = dfs[completecases(dfs), :];

site_locations = map((v) -> tuple(parse.(Float64,v)...),split.(chop.(dfs.location,head=1,tail=1),","));
munich_center = (48.1381,11.5759);
dfs.center_distance = haversine.(Ref(munich_center),site_locations, 6371.0088);

get_location_array = (df) -> begin 
    location = map((v) -> tuple(parse.(Float64,v)...)
             ,split.(chop.(df.location,head=1,tail=1),","));
    return hcat(collect.(location)...)
end







part_a = @linq dfs |> where(:center_distance .<= split_distance) 
locations_a = get_location_array(part_a)
centers_a = cluster_kmedian(locations_a,inner_count)
part_b = @linq dfs |> where(:center_distance .> split_distance)
locations_b =  get_location_array(part_b)
centers_b = cluster_kmedian(locations_b,outer_count)




if false
    using Plots
    using StatsPlots
    print("Plotting")
    plt=plot(legend=:topleft)
    #scatter!((last.(site_locations),first.(site_locations)), color=:green, label="Sites");
    scatter!(locations_a[2,centers_a],locations_a[1,centers_a], color=:red, label="Sites");
    scatter!(locations_b[2,centers_b],locations_b[1,centers_b], color=:blue, label="Sites");

    display(plt)
end

combined_df = vcat(part_a[[centers_a...],:],part_b[[centers_b...],:])

CSV.write("/dev/stdout",select!(combined_df, Not(:center_distance)))