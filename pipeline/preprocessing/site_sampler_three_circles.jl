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
    sites = "/home/gregor/Code/et/pipeline/input_data/taxi_sites.csv"
    split_distance_1 = 2
    split_distance_2 = 3

    inner_count = 10
    middle_count = 10
    outer_count = 40
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
    "--middle-circle-size"
        help = "Size of middle circle in km"
        arg_type = Float16
        required = true

    "--inner-count"
        help = "Number of sites in inner cirlce"
        arg_type = Int
        required = true
    "--middle-count"
        help = "Number of sites in middle cirlce"
        arg_type = Int
        required = true 
    "--outer-count"
        help = "Number of sites in outer circle"
        arg_type = Int
        required = true
end
args = parse_args(s)
sites = args["sites"]
split_distance_1 = args["inner-circle-size"]
split_distance_2 = args["middle-circle-size"]
inner_count = args["inner-count"]
middle_count = args["middle-count"]
outer_count = args["outer-count"]



function cluster_kmedoids(locations,count)
    distances = pairwise(Euclidean(), locations)
    kmedoids(distances,count,init=:kmpp).medoids
end

function cluster_kmedian(locations,count)
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

function get_location_array(df)
    location = map((v) -> tuple(parse.(Float64,v)...)
             ,split.(chop.(df.location,head=1,tail=1),","));
    return hcat(collect.(location)...)
end


part_a = @linq dfs |> where(:center_distance .<= split_distance_1) 
locations_a = get_location_array(part_a)
part_b = @linq dfs |> where(:center_distance .> split_distance_1) |> where(:center_distance .<= split_distance_2)
locations_b =  get_location_array(part_b)
part_c = @linq dfs |> where(:center_distance .> split_distance_2)
locations_c =  get_location_array(part_c)


# shift to next ring if current ring has not enough members
if length(locations_a)/2 < inner_count
    middle_count = middle_count + (inner_count - length(locations_a)/2)
    inner_count = length(locations_a)/2
end

# shift to next ring if current ring has not enough members
if length(locations_b)/2 < middle_count
    outer_count = outer_count + (middle_count - length(locations_b)/2)
    middle_count = length(locations_b)/2
end

# shift to first if still to many!
if length(locations_c)/2 < outer_count
    diff = outer_count - length(locations_c)/2
    if (inner_count + diff) <= length(locations_a)/2
        inner_count = inner_count + diff
    else
        exit(1)
    end
end

centers_a = if inner_count > 0  cluster_kmedian(locations_a,Int(inner_count))  else [] end
centers_b = if middle_count > 0  cluster_kmedian(locations_b,Int(middle_count)) else [] end
centers_c = if outer_count > 0  cluster_kmedian(locations_c,Int(outer_count)) else  [] end


if false
    using Plots
    using StatsPlots
    print("Plotting")
    plt=plot(legend=:topleft)
    #scatter!((last.(site_locations),first.(site_locations)), color=:green, label="Sites");
    scatter!(locations_a[2,centers_a],locations_a[1,centers_a], color=:red, label="Sites");
    scatter!(locations_b[2,centers_b],locations_b[1,centers_b], color=:blue, label="Sites");
    scatter!(locations_c[2,centers_c],locations_c[1,centers_c], color=:yellow, label="Sites");

    display(plt)
end

combined_df = vcat(
    part_a[[centers_a...],:],
    part_b[[centers_b...],:],
    part_c[[centers_c...],:]
    )

CSV.write("/dev/stdout",select!(combined_df, Not(:center_distance)))