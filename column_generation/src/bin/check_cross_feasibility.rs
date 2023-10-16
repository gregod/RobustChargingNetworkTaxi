use shared::{Site, Segment, Vehicle, Battery};




extern crate colored;


use rust_hawktracer::*;

use column_generation::fixed_size::check_feasibility::{CheckFeasibility};
use clap::{App, Arg};
use column_generation::fixed_size::brancher::SolveError::VehiclesInfeasible;


/// Tests whether a given site plan is infeasible when using the
/// capacities given

pub fn main() {


    let instance = HawktracerInstance::new();
    let _listener =  instance.create_listener(HawktracerListenerType::TCP {
        port: 12345,
        buffer_size: 4096,
    });

    let matches = App::new("Benders")
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

        .arg( Arg::with_name("percent_infeasible_allowed")
            .long("percent_infeasible_allowed")
            .default_value("0.0")
        )

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
            .help("Site Solution File to load")
            .required(true)
            .takes_value(true))
        .get_matches();


    let sites_path = matches.value_of("sites").unwrap();
    let battery_path = matches.value_of("battery").unwrap();


    let sites = Site::load(sites_path);
    let battery = Battery::load(battery_path);

    for (vehicles_path,trips_path) in matches.values_of("vehicles").unwrap().zip(matches.values_of("trips").unwrap()) {



        let segments = Segment::load(&sites, trips_path);
        let vehicles = Vehicle::load(&segments,vehicles_path,&battery);

        eprintln!("Started Checking Vehicle {} with trips {}",vehicles_path, trips_path);

        let num_infeasible_allowed = ((matches.value_of("percent_infeasible_allowed").unwrap().parse::<f64>().unwrap()) * vehicles.len() as f64).round() as usize;

        match CheckFeasibility::has_feasibility_error(&sites, &segments, &vehicles,num_infeasible_allowed)  {
            None => {
                println!("{}|{}|FEASIBLE|OK|{}|{}", vehicles_path,trips_path,0,vehicles.len());
            },
            Some(solve_error) => {
                match &solve_error {
                    VehiclesInfeasible(inf_vehicles) => {
                        println!("{}|{}|NOT_FEASIBLE|{:?}|{}|{}", vehicles_path,trips_path,solve_error,inf_vehicles.len(),vehicles.len());
                    },
                    _ => {
                        println!("{}|{}|NOT_FEASIBLE|{:?}|0|0", vehicles_path,trips_path,solve_error);
                    }
                }
            }

        }


    }


}
