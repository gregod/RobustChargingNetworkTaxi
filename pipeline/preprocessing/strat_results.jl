using CSV,CodecZlib, Mmap
using DataFrames
using DataFramesMeta
using Dates
using Plots
using StatsPlots



path = "/home/gregor/Code/et/pipeline/work"
current_instance = "$(path)/opt/1/group_1/battery_1/tol00/circle_4_50_10/500"
opt_log =  "$(current_instance)/opt_log"
active_sites = "$(current_instance)/active_sites.csv"


input_vehicles = "$(path)/preprocessed/group_1/circle_4_50_10/battery_1/feasible.vehicles.csv.gz"




const r_cost= r"^Solution: (\d*)"
const r_solution_sites = r"^Solution Sites: \[([^\]]*)\]"
const r_duration = r"^Duration: (\d*)s"

function get_score(opt_log_filename)
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

    return (duration, cost)
end

function get_feasible(input)

end

get_score(opt_log)



dfs = DataFrame(CSV.File(active_sites,ignoreemptylines = true));


dfp.RTime = dfp.Time .% 288
sort!(dfp, [:Vehicle, :RTime]);

sites = unique(dfp.Site)
myplots=[]
for site in groupby(dfp,:Site)
    shape = combine(groupby(site,:RTime), nrow)
    s = trunc(first(site.Site))
   
    sort!(shape,:RTime)
    push!(myplots,bar(shape.RTime,shape.nrow,title="s$(s)",c=:blue,lc=:blue,legend=false,ylim=(0,4),xlim=(0,288)))
end

display(plot(myplots...,size=(800,400)))

savefig("/tmp/plot_utilisation.pdf")