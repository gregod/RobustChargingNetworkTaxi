using CSV,CodecZlib, Mmap
using DataFrames
using DataFramesMeta
using Dates
using Plots
using StatsPlots
using KernelDensity


group = "2"
seed = "1"
vehicles = "1000"
numsite = "f50"


PERIODS_PER_DAY=288

active_sites = "/home/gregor/Code/et/pipeline/work/opt/1/group_0/battery_1/tol00/old_f60/1000/active_sites.csv"
charge_process = "/home/gregor/Code/et/pipeline/work/opt/1/group_0/battery_1/tol00/old_f60/1000/opt_chargeprocess"




dfs = DataFrame(CSV.File(active_sites,ignoreemptylines = true));
dfp = DataFrame(CSV.File(charge_process,ignoreemptylines = true,header=["Vehicle","Site","Time"]));


dfp.RTime = dfp.Time .% PERIODS_PER_DAY
sort!(dfp, [:Vehicle, :RTime]);

sites = unique(dfp.Site)
utilisations = []

myplots=[]
for site in groupby(dfp,:Site)
    shape = combine(groupby(site,:RTime), nrow)
    s = trunc(first(site.Site))
    site_capacity = first(dfs.capacity[dfs.id .== "s$(s)"])
    utilisation=sum(shape.nrow)/(PERIODS_PER_DAY*site_capacity)
    push!(utilisations,utilisation)
    sort!(shape,:RTime)
    push!(myplots,bar(shape.RTime,shape.nrow,title="s$(s) ($(round(utilisation*100))%)", titlefontsize=10,c=:blue,lc=:blue,legend=false,ylim=(0,4),xlim=(0,288)))
end

plt_all = plot(myplots...,size=(800,400))
savefig(plt_all,"/tmp/plot_utilisation.pdf")


plt_hist = histogram(utilisations*100,title="Histogram of utilisation rates",label="# Sites with Utilisation",legend=false,ylabel="# Sites",xlabel="Utilisation [%]",xlim=(0,100))
savefig(plt_hist,"/tmp/plot_utilisation_hist.pdf")

