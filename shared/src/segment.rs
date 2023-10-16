use std::cmp::Ordering;

use std::hash::{Hash, Hasher};

use crate::location::Location;
use crate::reachable_site::ReachableSite;
use crate::site::Site;
use crate::{get_reader, Period};

use indexmap::IndexMap;
use regex::Regex;


#[derive(Debug)]
pub struct Segment<'a> {
    pub id: u32,

    pub start_location: Location,
    pub stop_location: Location,
    pub distance: u32,

    pub start_time: Period,
    pub stop_time: Period,

    pub is_free: bool,

    pub reachable_sites: Vec<ReachableSite<'a>>,
}

impl<'a> Segment<'a> {
    pub fn load(taxi_sites: &'a IndexMap<u8, Site>, path: &str) -> IndexMap<u32, Segment<'a>> {
        let mut trips = IndexMap::default();
        let mut rdr = csv::Reader::from_reader(get_reader(path));
        let header_row = rdr.headers().unwrap();

        // get the ids for the relevant columns!
        let trip_id_column = header_row.iter().position(|x| x == "id").unwrap();
        let is_free_column = header_row.iter().position(|x| x == "isFree").unwrap();
        let start_time_column = header_row.iter().position(|x| x == "startPeriod").unwrap();
        let end_time_column = header_row.iter().position(|x| x == "endPeriod").unwrap();
        let distance_column = header_row.iter().position(|x| x == "osmDistance").unwrap();
        let start_point_column = header_row.iter().position(|x| x == "startPoint").unwrap();
        let end_point_column = header_row.iter().position(|x| x == "endPoint").unwrap();
        let potential_sites_column = header_row
            .iter()
            .position(|x| x == "potentialSites")
            .unwrap();

        // regex to extract the string of potential sites into sensible object
        let potential_site_regex =
            Regex::new(r"^s(\d*)\[(\d*)\|([\d\.]*)\|(\d*)\|([\d\.]*)\]").unwrap();

        for result in rdr.records() {
            let record = result.unwrap();

            let trip_id = record
                .get(trip_id_column)
                .unwrap()
                .trim_start_matches('t')
                .parse::<u32>()
                .expect("Could not parse segment id");
            let is_free = record.get(is_free_column).unwrap().to_ascii_lowercase() == "true";
            let start_time = record
                .get(start_time_column)
                .unwrap()
                .parse::<Period>()
                .expect("Could not parse start time period of segment");
            let stop_time = record
                .get(end_time_column)
                .unwrap()
                .parse::<Period>()
                .expect("Could not parse end time period of segment");

            debug_assert!(
                stop_time >= start_time,
                "Stop Time is smaller than start in trip {}",
                trip_id
            );

            let distance = record
                .get(distance_column)
                .unwrap()
                .parse::<f32>()
                .expect("Could not parse distance")
                .round() as u32;

            let start_point = record.get(start_point_column).unwrap();
            let end_point = record.get(end_point_column).unwrap();

            let potential_sites = record.get(potential_sites_column).unwrap();

            let potential_sites: Vec<ReachableSite> = potential_sites
                .split(';')
                .filter(|x| x != &"")
                .map(|x| x.trim_matches(' '))
                .map(|x| {
                    let captures = potential_site_regex.captures_iter(x).next();
                    match captures {
                        Some(capture) => {
                            let site_id: u8 = capture[1]
                                .parse()
                                .expect("Could not parse potential site id");
                            let arrival_period: Period = capture[2]
                                .parse()
                                .expect("Could not parse potential arrival period");
                            let arrival_distance = capture[3]
                                .parse::<f32>()
                                .expect("Could not parse potential arrival distance")
                                .round() as u32;
                            let departure_period: Period = capture[4]
                                .parse()
                                .expect("Could not parse potential departure period");
                            let departure_distance = capture[5]
                                .parse::<f32>()
                                .expect("Could not parse potential departure distance")
                                .round()
                                as u32;

                            ReachableSite {
                                site: taxi_sites.get(&site_id).unwrap(),
                                arrival_time: arrival_period,
                                departure_time: departure_period,
                                distance_to: arrival_distance,
                                distance_from: departure_distance,
                            }
                        }
                        None => panic!("Invalid Site: {}", x),
                    }
                })
                .collect();

            let patterns: &[_] = &['[', ']', ','];
            let location_points: Vec<_> = start_point
                .trim_matches(patterns)
                .split(',')
                .map(|x| x.trim_matches(' ').parse::<f32>().unwrap())
                .collect();

            let start_location = Location::new(location_points[0], location_points[1]);

            let location_points: Vec<_> = end_point
                .trim_matches(patterns)
                .split(',')
                .map(|x| x.trim_matches(' ').parse::<f32>().unwrap())
                .collect();
            let stop_location = Location::new(location_points[0], location_points[1]);

            let trip = Segment {
                id: trip_id,
                start_time,
                stop_time,
                is_free,
                start_location,
                stop_location,
                reachable_sites: potential_sites,
                distance,
            };

            trips.insert(trip_id, trip);
        }

        trips
    }
}

impl<'a> Eq for Segment<'a> {}

impl<'a> Hash for Segment<'a> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        state.write_u32(self.id);
        state.finish();
    }
}

impl<'a> PartialEq for Segment<'a> {
    fn eq(&self, other: &Segment) -> bool {
        self.id == other.id
    }
}

impl<'a> Ord for Segment<'a> {
    fn cmp(&self, other: &Segment) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<'a> PartialOrd for Segment<'a> {
    fn partial_cmp(&self, other: &Segment) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
