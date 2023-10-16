use shared::{Vehicle, Site, Segment, Period};
use std::fmt::{Formatter, Debug};

#[derive(Clone)]
pub enum BranchingFilter<'a> {
    ChargeSegmentSiteTime(&'a Vehicle<'a>, &'a Segment<'a>, &'a Site, Period, bool),
    ChargeSegmentSite(&'a Vehicle<'a>, &'a Segment<'a>, &'a Site, bool),
    OpenSite(&'a Site, bool),
    OpenSiteGroupMin(Vec<&'a Site>, f64),
    OpenSiteGroupMax(Vec<&'a Site>, f64),
}

impl<'a> Debug for BranchingFilter<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            BranchingFilter::ChargeSegmentSiteTime(vehicle, segment, site, period, typ) => {
                f.write_fmt(format_args!("SegmentSiteTime(v{},{},s{},{},{})", vehicle.id, segment.id,site.id,period,typ))
            },
            BranchingFilter::ChargeSegmentSite(vehicle, segment, site, typ) => {
                f.write_fmt(format_args!("SegmentSite(v{},{},s{},{})", vehicle.id,segment.id,site.id,typ))
            },
            BranchingFilter::OpenSite(site,  typ) => {
                f.write_fmt(format_args!("OpenSite(idx{},{})", site.index,typ))
            },
            BranchingFilter::OpenSiteGroupMax(sites, num) => {
                f.write_fmt(format_args!("OpenSiteGroupMax({:?},{})", sites.iter().map(|site| site.id).collect::<Vec<_>>(),num))
            },
            BranchingFilter::OpenSiteGroupMin(sites, num) => {
                f.write_fmt(format_args!("OpenSiteGroupMin({:?},{})", sites.iter().map(|site| site.id).collect::<Vec<_>>(),num))
            }
        }
    }
}