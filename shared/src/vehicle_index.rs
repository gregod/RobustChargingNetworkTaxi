use crate::battery::Battery;
use crate::get_reader;
use csv::Writer;

use indexmap::IndexMap;
use rand::Rng;
use std::cmp::Ordering;

use std::hash::{Hash, Hasher};
use std::io;

use crate::SegmentIndex;

#[derive(Debug, Clone)]
pub struct VehicleIndex<'a> {
    pub index: usize,
    pub original_id: u32,
    pub id: u32,
    pub battery: Battery,
    pub tour: Vec<&'a SegmentIndex>,
}

impl<'a> VehicleIndex<'a> {
    pub fn battery_initial_soc(&self) -> f64 {
        self.battery.initial_charge
    }
    pub fn battery_max_soc(&self) -> f64 {
        self.battery.max_charge
    }
    pub fn battery_min_soc(&self) -> f64 {
        self.battery.min_charge
    }
    pub fn battery_min_final_soc(&self) -> f64 {
        self.battery.min_final_charge
    }

    pub fn get_new_soc_after_distance(&self, current_soc: f64, distance_meters: u32) -> f64 {
        let factor: f64 = 100.0 / 1000.0 / 100.0 / self.battery.range_in_km;
        current_soc - (distance_meters as f64 * factor)
    }

    pub fn get_new_soc_after_charging(&self, current_soc: f64, duration_minutes: u8) -> f64 {
        let time_index = (((self.battery.soc_to_time[0])
            .mul_add(current_soc, self.battery.soc_to_time[1]))
        .mul_add(current_soc, self.battery.soc_to_time[2]))
        .mul_add(current_soc, self.battery.soc_to_time[3]);

        let new_index = time_index + (duration_minutes as f64);

        let new_soc = (((self.battery.time_to_soc[0])
            .mul_add(new_index, self.battery.time_to_soc[1]))
        .mul_add(new_index, self.battery.time_to_soc[2]))
        .mul_add(new_index, self.battery.time_to_soc[3]);

        let new_soc = new_soc
            .min(self.battery_max_soc())
            .max(self.battery_min_soc());

        debug_assert!(new_soc >= self.battery_min_soc());
        debug_assert!(new_soc <= self.battery_max_soc());

        new_soc
    }

    pub fn load(
        segments: &'a IndexMap<u32, SegmentIndex>,
        path: &str,
        battery: &Battery,
    ) -> Vec<Self> {
        let mut vehicles = Vec::new();

        let mut rdr = csv::Reader::from_reader(get_reader(path));

        let header_row = rdr.headers().unwrap();

        // get the ids for the relevant columns!
        let vehicle_id_column = header_row.iter().position(|x| x == "id").unwrap();
        let trips_column = header_row.iter().position(|x| x == "trips").unwrap();

        let mut index_counter = 0;

        for result in rdr.records() {
            let record = result.unwrap();

            let vehicle_id = record
                .get(vehicle_id_column)
                .unwrap()
                .trim_start_matches('v')
                .parse::<u32>()
                .unwrap();
            let track_ids_string = record.get(trips_column).unwrap();

            let patterns: &[_] = &['[', ']'];
            let track_ids: Vec<&SegmentIndex> = track_ids_string
                .trim_matches(patterns)
                .split(',')
                .filter(|x| x != &"")
                .map(|x| {
                    x.trim_matches(' ')
                        .trim_matches('t')
                        .parse::<u32>()
                        .unwrap()
                })
                .map(|x| segments.get(&x).unwrap())
                .collect();

            let vehicle = Self {
                original_id: vehicle_id,
                id: rand::thread_rng().gen::<u32>(),
                index: index_counter,
                tour: track_ids,
                battery: battery.clone(),
            };

            vehicles.push(vehicle);
            index_counter += 1;
        }
        vehicles
    }

    pub fn output<T>(vehicles: &[VehicleIndex<'a>], output: T)
    where
        T: io::Write,
    {
        let mut wtr = Writer::from_writer(output);
        wtr.write_record(&["index", "id", "trips"]).unwrap();
        for vehicle in vehicles {
            wtr.write_record(&[
                format!("{}", vehicle.index),
                format!("v{}", vehicle.original_id),
                format!(
                    "[ {} ]",
                    vehicle
                        .tour
                        .iter()
                        .map(|t| format!("t{}", t.id))
                        .collect::<Vec<String>>()
                        .join(",")
                ),
            ])
            .unwrap();
        }
        wtr.flush().unwrap();
    }
}

impl<'a> Ord for VehicleIndex<'a> {
    fn cmp(&self, other: &VehicleIndex) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<'a> PartialOrd for VehicleIndex<'a> {
    fn partial_cmp(&self, other: &VehicleIndex) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Eq for VehicleIndex<'a> {}

impl<'a> Hash for VehicleIndex<'a> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        state.write_u32(self.id);
        state.finish();
    }
}

impl<'a> PartialEq for VehicleIndex<'a> {
    fn eq(&self, other: &VehicleIndex) -> bool {
        self.id == other.id
    }
}
