using CSV,CodecZlib, Mmap
using DataFrames
using DataFramesMeta
using Dates
using Plots
using StatsPlots
using KernelDensity


group = "6"
seed = "1"
vehicles = "1000"
numsite = "f50"
stype="median"

old_strategies = [
    "old_f30",
    "old_f40",
    "old_f50",
    "old_f60"
]

strategies = [
"kde_60",
"circle_2_10_50",
"circle_2_20_40",
"circle_2_30_30",
"circle_2_40_20",
"circle_2_50_10",
"circle_3_10_50",
"circle_3_20_40",
"circle_3_30_30",
"circle_3_40_20",
"circle_3_50_10",
"circle_4_10_50",
"circle_4_20_40",
"circle_4_30_30",
"circle_4_40_20",
"circle_4_50_10",
"circle_5_10_50",
"circle_5_20_40",
"circle_5_30_30",
"circle_5_40_20",
"circle_5_50_10",
"threecircle_1-2_10-10-40",
"threecircle_1-2_20-10-30",
"threecircle_1-2_30-10-20",
"threecircle_1-2_40-10-10",
"threecircle_1-2_10-20-30",
"threecircle_1-2_20-20-20",
"threecircle_1-2_30-20-10",
"threecircle_1-2_40-10-10",
"threecircle_1-3_10-10-40",
"threecircle_1-3_20-10-30",
"threecircle_1-3_30-10-20",
"threecircle_1-3_40-10-10",
"threecircle_1-3_10-20-30",
"threecircle_1-3_20-20-20",
"threecircle_1-3_30-20-10",
"threecircle_1-3_40-10-10",
"threecircle_1-4_10-10-40",
"threecircle_1-4_20-10-30",
"threecircle_1-4_30-10-20",
"threecircle_1-4_40-10-10",
"threecircle_1-4_10-20-30",
"threecircle_1-4_20-20-20",
"threecircle_1-4_30-20-10",
"threecircle_1-4_40-10-10",
"threecircle_2-3_10-10-40",
"threecircle_2-3_20-10-30",
"threecircle_2-3_30-10-20",
"threecircle_2-3_40-10-10",
"threecircle_2-3_10-20-30",
"threecircle_2-3_20-20-20",
"threecircle_2-3_30-20-10",
"threecircle_2-3_40-10-10",
"threecircle_2-4_10-10-40",
"threecircle_2-4_20-10-30",
"threecircle_2-4_30-10-20",
"threecircle_2-4_40-10-10",
"threecircle_2-4_10-20-30",
"threecircle_2-4_20-20-20",
"threecircle_2-4_30-20-10",
"threecircle_2-4_40-10-10",
"threecircle_3-4_10-10-40",
"threecircle_3-4_20-10-30",
"threecircle_3-4_30-10-20",
"threecircle_3-4_40-10-10",
"threecircle_3-4_10-20-30",
"threecircle_3-4_20-20-20",
"threecircle_3-4_30-20-10",
"threecircle_3-4_40-10-10"
]



const r_cost= r"^Solution: (\d*)"
const r_solution_sites = r"^Solution Sites: \[([^\]]*)\]"
const r_duration = r"^Duration: (\d*)s"

function get_score(strat)
    opt_log_filename = "/home/gregor/Code/et/pipeline/work/opt/robust/battery_1/$(strat)/500/full_opt_log_quorum:100"
    lines = open(readlines, `tail -n 10 $(opt_log_filename)`)
    duration  = -1
    cost = -1

    for line in lines
        
        if startswith(line,"Total Number of Columns: ")
        elseif startswith(line,"Solution: ")
            cost = parse(Int,match(r_cost,line)[1])
        elseif startswith(line,"Solution Sites:")
            
        elseif startswith(line,"Duration: ")
            duration = parse(Int,match(r_duration,line)[1])
        end

    end
    
    return (duration=duration, cost=cost)
end



function get_infeasible(strat)
    input = "/home/gregor/Code/et/pipeline/work/opt/robust/battery_1/$(strat)/500/full_percent_feasible_quorum:100"
    return round(parse(Float64,open(readlines,input)[1])*100,digits=2)
end

function  plot_sites(strat::String)
    input = "/home/gregor/Code/et/pipeline/work/opt/robust/battery_1/$(strat)/500/full_active_sites_quorum:100.csv"
    dfs = DataFrame(CSV.File(input,ignoreemptylines = true));

    dfs_open = @linq dfs |> where(:capacity .> 0)  |> where(:cost .> 11);;
    dfs_closed = @linq dfs |> where(:capacity .== 0)  |> where(:cost .> 11);;


    sites_open = map((v) -> tuple(parse.(Float64,v)...)
                ,split.(chop.(dfs_open.location,head=1,tail=1),","));

    sites_closed = map((v) -> tuple(parse.(Float64,v)...)
                ,split.(chop.(dfs_closed.location,head=1,tail=1),","));
                
    plt = plot(legend=false,title="strat:$(strat), $(length(sites_open)) open, cost=$(get_score(strat).cost), inf=$(get_infeasible(strat))",titlefont = font(8))
    scatter!((last.(sites_open),first.(sites_open)), color=:green, label="Open Sites",ms=3);
    scatter!((last.(sites_closed),first.(sites_closed)), color=:red, label="Closed Sites",ms=2);
    
    
    return plt
end


# sort strategies by lowest cost
sort!(strategies,by=s -> get_score(s).cost)
plt = plot(map(plot_sites,strategies)...,size=(1500,700),link = :all)
display(plt)
savefig(plt,"/tmp/plots.png")



cost_values = map(x -> get_score(x).cost, strategies)
inf_values = map(x -> get_infeasible(x), strategies)

cost_values_old = map(x -> get_score(x).cost, old_strategies)
inf_values_old = map(x -> get_infeasible(x), old_strategies)

nondominates(arrx, arry) = begin
    return map(row -> begin
        return !any((arrx .< row[1]) .& (arry .<= row[2]))
    end,zip(arrx,arry))
end

cost_values


nond = nondominates(cost_values,inf_values)

plt2 = scatter(
    cost_values,
    inf_values,
    label="New Strategies",
    xlabel="Cost",
    ylabel="% Infeasible"
)

scatter!(
    cost_values_old,
    inf_values_old,
    label="Old Strategies",
    color=:red,
    series_annotations = text.(old_strategies, Ref(font(8,:bottom)))
)

plot!(
    cost_values[nond],
    inf_values[nond],
    label=:none,
    color=:blue,
    #series_annotations = text.(strategies[nond], Ref(font(8,:right)))
)



display(plt2)
savefig(plt2,"/tmp/pareto.png")