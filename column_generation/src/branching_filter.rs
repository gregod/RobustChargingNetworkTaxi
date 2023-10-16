use shared::{Vehicle, Site, Segment, Period};
use std::fmt::{Formatter, Debug};
use std::hash;
use crate::CG_EPSILON;
use crate::fixed_size::cg_model::{SegmentId, SiteIndex, VehicleIndex};
use crate::pattern_pool::Pattern;


#[derive(Clone,Hash,Eq,PartialEq)]
pub enum BranchingFilter {
    ChargeSegmentSiteTime(VehicleIndex, SegmentId, SiteIndex, Period, bool),
    ChargeSegmentSite(VehicleIndex, SegmentId, SiteIndex, bool),
    OpenSite(SiteIndex, bool),
    OpenSiteGroupMin(Vec<SiteIndex>, DataFloat),
    OpenSiteGroupMax(Vec<SiteIndex>, DataFloat),
    MasterNumberOfCharges(SiteIndex, Period, Dir, DataFloat),
    MasterMustUseColumn(VehicleIndex, Pattern, bool)
}

#[derive(Debug,Clone,Hash,Eq,PartialEq)]
pub enum Dir {
    Greater,
    Less
}

// https://stackoverflow.com/a/39647997
// F64 that is hashable
// we use them as constant data so issues with NaN are not a problem

#[derive(PartialOrd,Debug,Clone)]
pub struct DataFloat(f64);

impl DataFloat {
    fn key(&self) -> u64 {
        self.0.to_bits()
    }

    pub fn float(&self) -> f64 {
        self.0
}

    pub fn epsilon() -> Self {
        DataFloat(CG_EPSILON)
    }

    pub fn zero() -> Self {
        DataFloat(0.0)
    }
}

impl From<f64> for DataFloat {
    fn from(value: f64) -> Self {
        DataFloat(value)
    }
}

impl Into<f64> for DataFloat {
    fn into(self) -> f64 {
        self.0
    }
}

impl hash::Hash for DataFloat {
    fn hash<H>(&self, state: &mut H)
        where
            H: hash::Hasher,
    {
        self.key().hash(state)
    }
}

impl PartialEq for DataFloat {
    fn eq(&self, other: &DataFloat) -> bool {
        self.key() == other.key()
    }
}
impl Eq for DataFloat {}




impl<'a> Debug for BranchingFilter {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            BranchingFilter::ChargeSegmentSiteTime(vehicle, segment, site, period, typ) => {
                f.write_fmt(format_args!("SegmentSiteTime(v{},{},s{},{},{})", vehicle.index(), segment.index(),site.index(),period,typ))
            },
            BranchingFilter::ChargeSegmentSite(vehicle, segment, site, typ) => {
                f.write_fmt(format_args!("SegmentSite(v{},{},s{},{})", vehicle.index(),segment.index(),site.index(),typ))
            },
            BranchingFilter::OpenSite(site,  typ) => {
                f.write_fmt(format_args!("OpenSite(idx{},{})", site.index(),typ))
            },
            BranchingFilter::OpenSiteGroupMax(sites, num) => {
                f.write_fmt(format_args!("OpenSiteGroupMax({:?},{})", sites.iter().map(|site| site.index()).collect::<Vec<_>>(),num.float()))
            },
            BranchingFilter::OpenSiteGroupMin(sites, num) => {
                f.write_fmt(format_args!("OpenSiteGroupMin({:?},{})", sites.iter().map(|site| site.index()).collect::<Vec<_>>(),num.float()))
            },
            BranchingFilter::MasterNumberOfCharges(site, period, direction, size) => {
                f.write_fmt(format_args!("MasterNumberOfCharges({}@{} {:?} {})", site.index(), period,direction, size.float()))
            }

            BranchingFilter::MasterMustUseColumn(vehicle, visits, must_use) => {
                f.write_fmt(format_args!("MasterMustUseColumn(v{} {})", vehicle.index(),must_use))
            }
        }
    }
}