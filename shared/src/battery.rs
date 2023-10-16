use crate::{get_reader};



use toml;

#[derive(Debug, Clone)]
pub struct Battery {
    pub range_in_km: f64,
    pub max_charge: f64,
    pub min_charge: f64,
    pub initial_charge: f64,
    pub min_final_charge: f64,
    pub soc_to_time: [f64; 4],
    pub time_to_soc: [f64; 4],
}




use std::io::Read;
use toml::Value;

impl Battery {
    pub fn load(path: &str) -> Battery {
        let mut config_toml = String::new();

        let mut file = get_reader(path);

        file.read_to_string(&mut config_toml).unwrap();
        let parsed_config = config_toml.parse::<Value>().unwrap();
        let soc_min = parsed_config["SOC_min"].as_float().unwrap();
        let soc_max = parsed_config["SOC_max"].as_float().unwrap();
        let soc_initial = parsed_config["SOC_initial"].as_float().unwrap();
        let soc_final = parsed_config["SOC_final"].as_float().unwrap();
        let range_in_km = parsed_config["range_in_km"].as_integer().unwrap() as f64;
        let charging_speed = parsed_config["charging_speed"].as_float().unwrap();
        let battery_size = parsed_config["battery_size"].as_float().unwrap();

        let time_to_soc_vec: Vec<f64> = parsed_config["time_to_soc"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_float().unwrap())
            .collect();
        let soc_to_time_vec: Vec<f64> = parsed_config["soc_to_time"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_float().unwrap())
            .collect();

        let mut time_to_soc = [0.0; 4];
        time_to_soc.copy_from_slice(&time_to_soc_vec[..4]);
        let mut soc_to_time = [0.0; 4];
        soc_to_time.copy_from_slice(&soc_to_time_vec[..4]);

        Battery::new(
            soc_min,
            soc_max,
            soc_initial,
            soc_final,
            range_in_km,
            charging_speed,
            battery_size,
            time_to_soc,
            soc_to_time,
        )
    }

    pub fn new(
        soc_min: f64,
        soc_max: f64,
        soc_initial: f64,
        soc_final: f64,
        range_in_km: f64,
        _charging_speed: f64,
        _battery_size: f64,
        time_to_soc: [f64; 4],
        soc_to_time: [f64; 4],
    ) -> Self {
        Battery {
            initial_charge: soc_initial,
            range_in_km: range_in_km,
            min_final_charge: soc_final,
            max_charge: soc_max,
            min_charge: soc_min,
            time_to_soc,
            soc_to_time,
        }
    }

    pub fn get_new_soc_after_distance(&self, current_soc: f64, distance_meters: u32) -> f64 {
        let factor: f64 = 100.0 / 1000.0 / 100.0 / self.range_in_km;
        current_soc - (distance_meters as f64 * factor)
    }


    pub fn get_new_soc_after_charging(&self, current_soc: f64, duration_minutes: u8) -> f64 {
        let time_index = (((self.soc_to_time[0])
            .mul_add(current_soc, self.soc_to_time[1]))
            .mul_add(current_soc, self.soc_to_time[2]))
            .mul_add(current_soc, self.soc_to_time[3]);

        let new_index = time_index + (duration_minutes as f64);

        let new_soc = (((self.time_to_soc[0])
            .mul_add(new_index, self.time_to_soc[1]))
            .mul_add(new_index, self.time_to_soc[2]))
            .mul_add(new_index, self.time_to_soc[3]);

        let new_soc = new_soc
            .min(self.max_charge)
            .max(self.min_charge);

        debug_assert!(new_soc >= self.min_charge);
        debug_assert!(new_soc <= self.max_charge);

        new_soc
    }
}
