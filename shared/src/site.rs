use crate::location::Location;
use indexmap::IndexMap;

use std::hash::{Hash, Hasher};

use crate::get_reader;



#[derive(Debug, Clone)]
pub struct Site {
    pub id: u8,
    pub index: usize,
    pub location: Location,
    pub cost: u8,
    pub charger_cost: u8,
    pub capacity: u8,
}

impl Hash for Site {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        state.write_u8(self.id);
        state.finish();
    }
}

impl Eq for Site {}

impl PartialEq for Site {
    fn eq(&self, other: &Site) -> bool {
        self.id == other.id
    }
}

impl Site {
    pub fn load(path: &str) -> IndexMap<u8, Site> {
        // read taxi sites
        let mut rdr = csv::Reader::from_reader(get_reader(path));

        let mut taxi_sites: IndexMap<u8, Site> = IndexMap::default();

        let mut index_counter = 0;
        //let mut charger_in_use = Vec::new();

        let header_row = rdr.headers().unwrap();

        // get the ids for the relevant columns!
        let site_id_column = header_row.iter().position(|x| x == "id").unwrap();
        let capacity_column = header_row.iter().position(|x| x == "capacity").unwrap();
        let cost_column = header_row.iter().position(|x| x == "cost").unwrap();
        let location_column = header_row.iter().position(|x| x == "location").unwrap();

        for result in rdr.records() {
            // Notice that we need to provide a type hint for automatic
            // deserialization.
            let record = result.unwrap();

            let site_id = record
                .get(site_id_column)
                .unwrap()
                .trim_start_matches('s')
                .parse::<u8>()
                .unwrap();
            let capacity = record.get(capacity_column).unwrap().parse::<u8>().unwrap();
            let cost = record.get(cost_column).unwrap().parse::<u8>().unwrap();
            let location = record.get(location_column).unwrap();

            let patterns: &[_] = &['[', ']'];
            let location_points: Vec<_> = location
                .trim_matches(patterns)
                .split(',')
                .map(|x| x.trim_matches(' ').parse::<f32>().unwrap())
                .collect();
            let location = Location::new(location_points[0], location_points[1]);

            let site = Site {
                id: site_id,
                index: index_counter,
                location,
                cost,
                charger_cost: (f32::from(cost) * 0.1).round() as u8,
                capacity,
            };

            taxi_sites.insert(site_id, site);
            index_counter += 1;
        }

        taxi_sites
    }
}
