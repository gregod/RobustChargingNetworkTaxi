using CSV,CodecZlib, Mmap
using DataFrames
using DataFramesMeta
using Dates
using Plots
using Plots.PlotMeasures
using StatsPlots
using KernelDensity
using Geodesy

vehicles = "1000"


function get_starts_ends(vehicles)

    println("Loading",vehicles)
    numsite = "f60"
    stype="median"

    all_starts = []
    all_ends = []
    for seed in [1,2,3]
        for group in [0,1,2,3,4,5,6]
            vehicle_input = "/home/gregor/Code/et/pipeline/work/preprocessed/$(seed)/group_$(group)/$(numsite)/$(vehicles)/battery_1.final.vehicles.csv.gz";
            trip_input = "/home/gregor/Code/et/pipeline/work/preprocessed/$(seed)/group_$(group)/$(numsite)/$(vehicles)/battery_1.final.trips.csv.gz";
            dfv = DataFrame(CSV.File(transcode(GzipDecompressor, Mmap.mmap(vehicle_input)),ignoreemptylines = true));
            dft = DataFrame(CSV.File(transcode(GzipDecompressor, Mmap.mmap(trip_input)),ignoreemptylines = true));
            dfv.tripsList = split.(chop.(dfv[:trips],head=2,tail=2),",");

            all_vehicle_trips = Set(reduce(vcat,dfv.tripsList));
            dft = dft[in.(dft.id, Ref(all_vehicle_trips)),:];
            dft_free = @linq dft |> where(:isFree .== 0)  |> select(:endPoint,:startPoint);
            dft_unfree = @linq dft |> where(:isFree .== 1)  |> select(:endPoint,:startPoint);

            starts = map((v) -> tuple(parse.(Float64,v)...)
                        ,split.(chop.(dft_free.startPoint,head=1,tail=1),","))

            ends = map((v) -> tuple(parse.(Float64,v)...)
                        ,split.(chop.(dft_unfree.endPoint,head=1,tail=1),","));

            append!(all_starts,starts)
            append!(all_ends,ends)
        end
    end


    return (
        starts=all_starts,
        ends=all_ends
    )

end

function project(x)
    return UTMZ(LLA(x[1],x[2],0),wgs84)
end

function points_to_tuple(input::Array{UTMZ{Float64},1})
    return (map(i -> i.x,input),map(i -> i.y,input))
end

function get_plot(vehicles)
    group = "1"
    seed = "1"
    numsite = "f60"
    stype="median"
    iis = "true"
    active_sites = "/home/gregor/Code/et/pipeline/work/opt/robust/battery_1/old_$(numsite)/$(vehicles)/lowest_active_sites_quorum:100_activate:1_benevolent:4_iis:$(iis).csv"
   
    dfs = DataFrame(CSV.File(active_sites,ignoreemptylines = true));

    dfs_open = @linq dfs |> where(:capacity .> 0) |> where(:cost .> 11);
    dfs_closed = @linq dfs |> where(:capacity .== 0) |> where(:cost .> 11);



    print("Rebuilding KDE")

    trip_data = get_starts_ends(vehicles)    
    projected_starts = map(project,trip_data.ends)

   bound1 = project((48.05,11.4))
   bound2 = project((48.23,11.75))

    
    k = kde(points_to_tuple(projected_starts), boundary=((bound1.x,bound2.x),(bound1.y,bound2.y)));

    heatmap(k,fillcolor=:thermal,legend=false, title="|V|=$(vehicles)");

    sites_open = map((v) -> tuple(parse.(Float64,v)...)
                ,split.(chop.(dfs_open.location,head=1,tail=1),","));

    sites_closed = map((v) -> tuple(parse.(Float64,v)...)
                ,split.(chop.(dfs_closed.location,head=1,tail=1),","));



    projected_sites_open = map(project,sites_open)
    projected_sites_closed = map(project,sites_closed)

    scatter!(points_to_tuple(projected_sites_open), color=:green, label="Open Sites");
    scatter!(points_to_tuple(projected_sites_closed), color=:red, label="Closed Sites");


    return plot!(xaxis=false,yaxis=false, left_margin = -12mm, right_margin= -2mm)
end


p500 = get_plot(500)
p1000 = get_plot(1000)
plt = plot(p500,p1000,size=(750,350),link = :all, bottom_margin = -2mm)
savefig(plt,"/tmp/plot_heatmap.pdf")
plt