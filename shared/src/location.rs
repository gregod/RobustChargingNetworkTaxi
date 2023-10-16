#![allow(dead_code)]
use std;

use std::f32;
use std::fmt;

#[derive(Clone)]
pub struct Location {
    pub lat: f32,
    pub lon: f32,
}

impl Location {
    pub fn new(lat: f32, lon: f32) -> Location {
        Location { lat: lat, lon: lon }
    }

    fn deg2rad(deg: f32) -> f32 {
        deg * (f32::consts::PI / 180.0)
    }

    pub fn rad2deg(rad: f32) -> f32 {
        rad * (180.0 / f32::consts::PI)
    }
}

impl fmt::Debug for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Location {{ lat: {}, lon: {} }}", self.lat, self.lon)
    }
}
