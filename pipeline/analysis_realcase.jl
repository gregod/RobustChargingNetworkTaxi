### A Pluto.jl notebook ###
# v0.12.16

using Markdown
using InteractiveUtils

# ╔═╡ d55971aa-3632-11eb-1198-850c3cb5834c
begin
	import GZip
	using Printf
	using Plots
	using Polynomials
	using Measures
	gr()
end

# ╔═╡ d4576aa8-3a03-11eb-36ad-61609ac7a3c9


# ╔═╡ 22cb8734-3633-11eb-183b-0dab29d4557a
prefix = "/home/gregor/Code/et/pipeline/work/preprocessed/realcase/"

# ╔═╡ deb9e83a-3969-11eb-28c4-9196386eab15
prefix_opt = "/home/gregor/Code/et/pipeline/work/opt/realcase/"

# ╔═╡ a0cea7e6-3634-11eb-1535-ff8f06bd8ca7
variation = [0.85,0.90,0.95,1,1.05,1.10,1.15]

# ╔═╡ 6416431e-3633-11eb-0968-77b1447e313b
function get_num_vehicles_in_base(dbat,dcha,dfin,day)
	
	fmt = @sprintf "%.2f_dcha:%.2f_dfin:%.2f.%02d" dbat dcha dfin day
	filename = "$(prefix)vehicles.base.dbat:$(fmt).csv.gz";
	fh = GZip.open(filename)
	s = countlines(fh)
	close(fh)
	return s
end

# ╔═╡ d3a50dc6-3969-11eb-2286-037a1095513a
function get_vehicle_inf(dbat,dcha,dfin)
	
	fmt = @sprintf "dbat:%.2f_dcha:%.2f_dfin:%.2f" dbat dcha dfin
	filename = "$(prefix_opt)robust/$(fmt)/lowest_percent_feasible_quorum:100_activate:1_benevolent:5_iis:true";
	s = readline(filename)
	return parse(Float64,split(s,"|")[1])
end

# ╔═╡ 1d2380de-3976-11eb-198e-db954e12fa01
function get_vehicle_score(dbat,dcha,dfin)
	
	fmt = @sprintf "dbat:%.2f_dcha:%.2f_dfin:%.2f" dbat dcha dfin
	filename = "$(prefix_opt)robust/$(fmt)/lowest_opt_log_quorum:100_activate:1_benevolent:5_iis:true";
	s = readlines(filename);
	prefix = "Solution: "
	parse(Int,chop(last(filter(e -> startswith(e,prefix),s)),head=length(prefix),tail=0))
	 	
	
end

# ╔═╡ 6a778662-3976-11eb-0e50-47f6da5abd92
get_vehicle_score(1,1,1)

# ╔═╡ 6f382fac-396a-11eb-15d4-3384d04caa68
begin
dbat_var = [ get_vehicle_inf(var,1,1) for var in variation]
dcha_var = [ get_vehicle_inf(1,var,1) for var in variation]
dfin_var = [ get_vehicle_inf(1,1,var) for var in variation]
end

# ╔═╡ 4ddb2880-3a09-11eb-3575-e78d36085600
var_base = get_vehicle_inf(1,1,1)

# ╔═╡ fa57d10c-3977-11eb-3039-29f6e87fee87
begin
dbat_var_cost = [ get_vehicle_score(var,1,1) for var in variation]
dcha_var_cost = [ get_vehicle_score(1,var,1) for var in variation]
dfin_var_cost = [ get_vehicle_score(1,1,var) for var in variation]
end

# ╔═╡ 2a5d65ac-3a0d-11eb-3c1f-e16e05461cd8
var_base_cost = get_vehicle_score(1,1,1)

# ╔═╡ 2357c3a0-3a04-11eb-24fd-216dccb0aa92
variation_as_pct_points = (variation.-1)

# ╔═╡ 14dac2e0-396a-11eb-2440-6f98cfb37471
plot_var = let 
	p = plot(xlabel="Parameter Variation [%]",ylabel="Δ Vehicle Infeasibility [pp]",legend=:topleft,
#	    bottom_margin = 7mm,
#	    left_margin=15mm
)
	plot!(p,variation_as_pct_points*100,(dbat_var .- var_base)*100,label = "Battery Capacity")
	plot!(p,variation_as_pct_points*100,(dcha_var .- var_base)*100,label = "Charger Power")
	plot!(p,variation_as_pct_points*100,(dfin_var .- var_base)*100,label = "Battery Requirement at Shift End")
	p
end

# ╔═╡ 465a3b6a-3b41-11eb-3374-2728fd367aec


# ╔═╡ 7b57dde6-3b40-11eb-2287-f5df14829629
savefig(plot_var,"/tmp/var_inf.pdf")

# ╔═╡ c63f5548-3a03-11eb-39de-ade594444d35
p_bat = coeffs(fit(variation_as_pct_points,dbat_var,1)) # fit first order polynomial

# ╔═╡ 2bf4a762-3a04-11eb-3376-d1f73c6ad4d7
p_char = coeffs(fit(variation_as_pct_points,dcha_var,1)) # fit first order polynomial

# ╔═╡ d66bda48-3a05-11eb-33ef-772d818cf553
p_fin = coeffs(fit(variation_as_pct_points,dfin_var,1)) # fit first order polynomial

# ╔═╡ 04edd1d4-3978-11eb-1444-bd18a5bf61af
plot_cost = let 
	p = plot(xlabel="Parameter Variation [%]",ylabel="Δ Cost [%]",legend=:topleft,
		    bottom_margin = 7mm,
		    left_margin=15mm,
		    fontfamily = "Computer Modern"
	)
	plot!(p,(variation.-1)*100,((dbat_var_cost/var_base_cost) .- 1) * 100,label = "Battery")
	plot!(p,(variation.-1)*100,((dcha_var_cost/var_base_cost) .- 1) * 100,label = "Charger")
	plot!(p,(variation.-1)*100,((dfin_var_cost/var_base_cost) .- 1) * 100,label = "Battery Requirement at End")
	p
end

# ╔═╡ fca4becc-3d9c-11eb-24f4-25d2401ae3b1
savefig(plot_cost,"/tmp/var_cost.pdf")

# ╔═╡ 97446f32-3a65-11eb-09a1-d1c461662ec0
plot(plot_var,plot_cost,legend=:none,share=:none)

# ╔═╡ Cell order:
# ╠═d4576aa8-3a03-11eb-36ad-61609ac7a3c9
# ╠═d55971aa-3632-11eb-1198-850c3cb5834c
# ╟─22cb8734-3633-11eb-183b-0dab29d4557a
# ╟─deb9e83a-3969-11eb-28c4-9196386eab15
# ╟─a0cea7e6-3634-11eb-1535-ff8f06bd8ca7
# ╟─6416431e-3633-11eb-0968-77b1447e313b
# ╟─d3a50dc6-3969-11eb-2286-037a1095513a
# ╟─1d2380de-3976-11eb-198e-db954e12fa01
# ╠═6a778662-3976-11eb-0e50-47f6da5abd92
# ╠═6f382fac-396a-11eb-15d4-3384d04caa68
# ╠═4ddb2880-3a09-11eb-3575-e78d36085600
# ╠═fa57d10c-3977-11eb-3039-29f6e87fee87
# ╠═2a5d65ac-3a0d-11eb-3c1f-e16e05461cd8
# ╠═2357c3a0-3a04-11eb-24fd-216dccb0aa92
# ╠═14dac2e0-396a-11eb-2440-6f98cfb37471
# ╠═465a3b6a-3b41-11eb-3374-2728fd367aec
# ╠═7b57dde6-3b40-11eb-2287-f5df14829629
# ╠═c63f5548-3a03-11eb-39de-ade594444d35
# ╠═2bf4a762-3a04-11eb-3376-d1f73c6ad4d7
# ╠═d66bda48-3a05-11eb-33ef-772d818cf553
# ╠═04edd1d4-3978-11eb-1444-bd18a5bf61af
# ╠═fca4becc-3d9c-11eb-24f4-25d2401ae3b1
# ╠═97446f32-3a65-11eb-09a1-d1c461662ec0
