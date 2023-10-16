use shared::{Site, Segment, Vehicle, Battery};




extern crate colored;
use colored::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool};

use column_generation::fixed_size::benders::Benders;
use std::time::{Instant};
use clap::{App, Arg};
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
            .takes_value(true))
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



    let vehicles_path = matches.value_of("vehicles").unwrap();
    let trips_path =  matches.value_of("trips").unwrap();
    let sites_path =matches.value_of("sites").unwrap();
    let battery_path =matches.value_of("battery").unwrap();
    let _duration = matches.value_of("duration").unwrap().parse::<u64>().unwrap();

    let num_sites = matches.value_of("min_num_sites").unwrap().parse::<usize>().unwrap();

    let sites = Site::load(sites_path);
    let segments = Segment::load(&sites, trips_path);
    let battery = Battery::load(battery_path);
    let vehicles = Vehicle::load(&segments,vehicles_path,&battery);


    let num_infeasible_allowed = ((matches.value_of("percent_infeasible_allowed").unwrap().parse::<f64>().unwrap()) * vehicles.len() as f64).round() as usize;


    eprintln!("{}","♞ Loading Data Completed".on_green().bold());

    eprintln!("Working with {} vehicles", vehicles.len());
    println!("Working with {} vehicles", vehicles.len());

    let should_stop = Arc::new(AtomicBool::new(false));


    let mut seq = Benders::new(
        num_sites,num_infeasible_allowed, sites.clone(), &vehicles
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


    let solution = seq.run(should_stop, matches.value_of("charge_processes_output").unwrap(),matches.value_of("cuts_output").unwrap(),matches.value_of("cuts_input").unwrap());

    eprintln!("{}","♞ Column Generation Completed".on_green().bold());

    println!("Solution: {}", solution.cost);
    println!("Solution Sites: {:?}", solution.sites_open);
    println!("Duration: {}s", start.elapsed().as_secs());

}
