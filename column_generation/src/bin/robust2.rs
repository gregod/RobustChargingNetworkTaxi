use shared::{Site, Segment, Vehicle, Battery};



extern crate colored;


use rust_hawktracer::*;


use clap::{App, Arg};
use indexmap::map::IndexMap;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;





extern crate rand;



use column_generation::fixed_size::benders_robust::BendersRobust;
use std::time::Instant;

/// Tests whether a given site plan is infeasible when using the
/// capacities given

pub fn main() {


    let instance = HawktracerInstance::new();
    let _listener =  instance.create_listener(HawktracerListenerType::TCP {
        port: 12345,
        buffer_size: 4096,
    });

    let matches = App::new("Robust")
        .arg(Arg::with_name("vehicles")
            .short("v")
            .long("vehicles")
            .value_name("FILE")
            .help("Vehicles to load")
            .required(true)
            .multiple(true)
            .min_values(1)
            .takes_value(true)
        )
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
            .help("Site Input File to load")
            .required(true)
            .takes_value(true)
        )
        .arg( Arg::with_name("cuts_input")
            .long("cuts_input")
            .default_value("/dev/null")
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


    let quorum_accept_percent : u8 = matches.value_of("quorum_accept_percent").unwrap().parse().expect("Invalid quorum percent");
    let benevolent_accept_percent : u8 = matches.value_of("benevolent_accept_percent").unwrap().parse().expect("Invalid benevolent_accept_percent");
    let max_activate_per_generation : usize = matches.value_of("max_activate_per_generation").unwrap().parse().expect("Invalid max_activate_per_generation");
    let total_num_vehicles : i64 = matches.value_of("total_num_vehicles").unwrap().parse().expect("Invalid total_num_feasible");

    let sites_path = matches.value_of("sites").unwrap();
    let battery_path = matches.value_of("battery").unwrap();


    let sites = Site::load(sites_path);
    let battery = Battery::load(battery_path);

    let should_stop = Arc::new(AtomicBool::new(false));

    let scenarios : Vec<(&str,&str)> = matches.values_of("vehicles").unwrap().zip(matches.values_of("trips").unwrap()).into_iter().collect();
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

    let mut bend = BendersRobust::new(
        &sites,
        &input_data,
        0, // is 0 as we never use this as criterion
                        // the actual infeasibility allowed is ensured through the
                        // benevolency parameter & determining the iis seperately
        quorum_accept_percent,
        benevolent_accept_percent,
        max_activate_per_generation,
        matches.is_present("activate_all"),
        matches.value_of("activate_iis").unwrap().parse::<bool>().unwrap(),
        total_num_vehicles

    );

    let start = Instant::now();

    let solution = bend.run(
            should_stop.clone(),"/dev/null",
            "/dev/null",matches.value_of("cuts_input").unwrap(),
    );

    println!("Solution: {}", solution.cost);
    println!("Solution Sites: {:?}", solution.sites_open);
    println!("Duration: {}s", start.elapsed().as_secs());


}
