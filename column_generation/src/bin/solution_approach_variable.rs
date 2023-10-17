use shared::{Site, Segment, Vehicle, Battery};

#[cfg(feature = "profiling_enabled")]
use std::path::PathBuf;


extern crate colored;
use colored::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool};

use column_generation::fixed_size::solution_approach_variable::SolutionApproachVariable;
use std::time::{Instant};
use clap::{App, Arg};
use grb::{Env,param};
use indexmap::IndexMap;
use itertools::Itertools;
#[cfg(feature = "profiling_enabled")]
use rust_hawktracer::{HawktracerInstance, HawktracerListenerType};


pub fn main() {

    #[cfg(feature = "perf_statistics")] {
        shared::setup_metrics_printer();
    }

    let matches = App::new("Benders")


        .arg(Arg::with_name("vehicles")
            .short("v")
            .long("vehicles")
            .value_name("FILE")
            .help("Vehicles to load")
                 .required(true)
                 .multiple(true)
                 .min_values(1)
                 .takes_value(true))

        .arg(Arg::with_name("battery")
            .short("b")
            .long("battery")
            .value_name("FILE")
            .help("Battery Config to load")
            .required(true)
            .takes_value(true))

        .arg(Arg::with_name("trips")
            .short("t")
            .long("trips")
            .value_name("FILE")
            .help("Trips to load")
                 .required(true)
                 .multiple(true)
                 .min_values(1)
                 .takes_value(true)
        )

        .arg(Arg::with_name("sites")
            .short("s")
            .long("sites")
            .value_name("FILE")
            .help("Sites to load")
            .required(true)
            .takes_value(true))

        .arg( Arg::with_name("duration")
            .long("duration")
            .default_value("3600")
        )

        .arg( Arg::with_name("workers")
            .long("workers")
            .env("SLURM_CPUS_PER_TASK")
            .default_value("1")
        )

        .arg( Arg::with_name("min_num_sites")
            .long("sites_min")
            .default_value("5")
        )


        .arg( Arg::with_name("percent_infeasible_allowed")
            .long("percent_infeasible_allowed")
            .default_value("0.0")
        )

        .arg( Arg::with_name("charge_processes_output")
            .long("charge_processes_file")
            .default_value("/dev/null")
        )
        .arg( Arg::with_name("cuts_output")
            .long("cuts_output")
            .default_value("/dev/null")
        )
        .arg( Arg::with_name("cuts_input")
            .long("cuts_input")
            .default_value("/dev/null")
            .takes_value(true)
        )

        .arg( Arg::with_name("hawktracer_output")
            .long("hawktracer_output")
            .default_value("/dev/null")
        )

        .arg( Arg::with_name("do_variable")
            .long("do_variable").conflicts_with_all(&["do_low", "do_high_low", "site_size"])
        )
        .arg( Arg::with_name("do_low")
            .long("do_low")
            .conflicts_with_all(&["do_variable", "do_high_low", "site_size"])
        )
        .arg( Arg::with_name("do_high_low")
            .long("do_high_low")
            .conflicts_with_all(&["do_variable", "do_low", "site_size"])
        )
        .arg( Arg::with_name("site_size")
                  .long("site_size")
                  .conflicts_with_all(&["do_variable", "do_low", "do_high_low"])
                  .takes_value(true)
        )

        .arg(Arg::with_name("quorum_accept_percent")
            .long("quorum_accept_percent")
            .help("Perctentage of oracles needed to agree on configuration")
            .default_value("100")
            .takes_value(true)
        )
        .arg(Arg::with_name("benevolent_accept_percent")
            .long("benevolent_accept_percent")
            .help("Percentage of invalid vehicles that is still considered feasible")
            .default_value("0")
            .takes_value(true)
        )
        .arg(Arg::with_name("max_activate_per_generation")
            .long("max_activate_per_generation")
            .help("Maximum number of scenarios to activate in solution")
            .default_value("1")
            .takes_value(true)
        )
        .arg(Arg::with_name("activate_all")
            .long("activate_all")
            .help("Should we activate all scenarios from the beginning")
        )
        .arg(Arg::with_name("activate_iis")
            .long("activate_iis")
            .default_value("false")
            .takes_value(true)
            .help("Should we activate only the iis vehicles in fresh scenarios?")
        )
        .arg(Arg::with_name("total_num_vehicles")
            .long("total_num_vehicles")
            .default_value("-1")
            .takes_value(true)
            .help("Use this value to calculate feasibility in benevolent feasibility check. Set to -1 to use real input number")
        )
        .get_matches();




    #[cfg(feature = "profiling_enabled")]
        let instance = HawktracerInstance::new();
    #[cfg(feature = "profiling_enabled")]
        let _listener = if matches.value_of("hawktracer_output").unwrap() == "/dev/null" {
            instance.create_listener(HawktracerListenerType::TCP {
                port: 12345,
                buffer_size: 4096,
            })
        } else {
            instance.create_listener(HawktracerListenerType::ToFile {
                file_path: PathBuf::from(matches.value_of("hawktracer_output").unwrap()),
                buffer_size: 4096,
            })
        };



    let sites_path =matches.value_of("sites").unwrap();
    let battery_path =matches.value_of("battery").unwrap();
    let _duration = matches.value_of("duration").unwrap().parse::<u64>().unwrap();


    let quorum_accept_percent : u8 = matches.value_of("quorum_accept_percent").unwrap().parse().expect("Invalid quorum percent");
    let benevolent_accept_percent : u8 = matches.value_of("benevolent_accept_percent").unwrap().parse().expect("Invalid benevolent_accept_percent");
    let max_activate_per_generation : usize = matches.value_of("max_activate_per_generation").unwrap().parse().expect("Invalid max_activate_per_generation");
    let total_num_vehicles : i64 = matches.value_of("total_num_vehicles").unwrap().parse().expect("Invalid total_num_feasible");


    let workers = matches.value_of("workers").unwrap().parse::<i32>().unwrap();

    let num_sites = matches.value_of("min_num_sites").unwrap().parse::<usize>().unwrap();

    let sites = Site::load(sites_path);
    let battery = Battery::load(battery_path);




    let do_low = matches.is_present("do_low");
    let mut do_variable = matches.is_present("do_variable");
    let mut do_high_low = matches.is_present("do_high_low");


    if let Some(size) = matches.value_of("site_size") {
        match size {
            "4" => {},
            "2" => { do_high_low = true }
            "variable" => { do_variable = true}
            _ => unreachable!("Not an option")
        }
    }



    let should_stop = Arc::new(AtomicBool::new(false));



    let scenarios : Vec<(&str,&str)> = matches.values_of("vehicles").unwrap().zip(matches.values_of("trips").unwrap()).into_iter()
        .unique() // keep only first on duplicates. is snakemake gives seed scenario first, then all scenarios again!
        .collect();
    println!("First is:, {:?}", &scenarios[0]);

    let all_segments = scenarios.iter().map(|(_,spath)| {
        Segment::load(&sites, spath)
    }).fold( IndexMap::new() as IndexMap<u32,Segment>, | mut agg,item| {
        agg.extend(item);
        agg
    });



    let scenarios_data : Vec<(IndexMap<u32,Segment>,Vec<Vehicle>)> = scenarios.into_iter().map(|(v,s)| {
        (
            Segment::load(&sites, s),
            Vehicle::load(&all_segments,v,&battery)
        )
    }).collect();



    let input_data : Vec<&[Vehicle]> = scenarios_data.iter().map(|(_s,v)| {
        v.as_slice()
    }).collect();


    let num_infeasible_allowed = ((matches.value_of("percent_infeasible_allowed").unwrap().parse::<f64>().unwrap()) * input_data[0].len() as f64).round() as usize;
    if num_infeasible_allowed > 0 {
        // only suitable for non robust setting
        assert_eq!(input_data.len(),1);
    }



    eprintln!("{}","♞ Loading Data Completed".on_green().bold());

    eprintln!("Working with {} scenarios", input_data.len());
    println!("Working with {} scenarios", input_data.len());

    eprintln!("Working with {} vehicles per scenario", input_data[0].len());
    println!("Working with {} vehicles per scenario", input_data[0].len());


    let mut env = Env::new("").unwrap();
    env.set(param::Threads, 1).unwrap();
    // 2= barrier; test with concurrent has shown that
    // this usually wins!
    //env.set(param::Method, 2).unwrap();
    env.set(param::Seed, 12345).unwrap();
    env.set(param::LogToConsole, 0).unwrap();


    // set low time limit; we mainly want the integer solution by branching this function is only for quick wins;
    let mut env_integer = Env::new("").unwrap();
    env_integer.set(param::Threads, 1).unwrap();
    env_integer.set(param::Seed, 12345).unwrap();
    env_integer.set(param::LogToConsole, 0).unwrap();
    env_integer.set(param::TimeLimit, 60.0).unwrap();


    let mut seq = SolutionApproachVariable::new(
        num_sites,num_infeasible_allowed, sites.clone(), &input_data, workers, true, quorum_accept_percent, benevolent_accept_percent, max_activate_per_generation,
        matches.is_present("activate_all"),
        matches.value_of("activate_iis").unwrap().parse::<bool>().unwrap(), total_num_vehicles,
        &env, &env_integer
    );




    let start = Instant::now();


    let _timer = timer::Timer::new();
    let _timer_should_stop = should_stop.clone();
    /*let timeout = chrono::Duration::seconds(matches.value_of("timeout").unwrap().parse::<i64>().unwrap());
    let _guard = timer.schedule_with_delay(timeout, move || {
        timer_should_stop.store(true,Relaxed);
        println!("TIMEOUT after {}",timeout);
        eprintln!("TIMEOUT after {}",timeout);
    });*/


    let solution = seq.run(should_stop, matches.value_of("charge_processes_output").unwrap(),matches.value_of("cuts_output").unwrap(),matches.value_of("cuts_input").unwrap(),
    do_low,do_high_low, do_variable
    );

    eprintln!("{}","♞ Column Generation Completed".on_green().bold());

    println!("Solution: {}", solution.cost);
    println!("Solution Sites: {:?}", solution.sites_open);
    println!("Duration: {}s", start.elapsed().as_secs());

}
