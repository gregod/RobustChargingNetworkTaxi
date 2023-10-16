use shared::Site;

pub type SiteConf = Vec<u8>;


pub struct  SiteConfFactory{
   pub num_sites : usize
}

impl SiteConfFactory {
    pub fn empty(&self) -> SiteConf{
        vec![0; self.num_sites]
    }
    pub fn full(&self,size : u8) -> SiteConf {
        vec![size; self.num_sites]
    }

    pub fn from_closed_vector(&self, closed_sites : &[bool], site_array : &[Site], site_cap : u8) -> SiteConf {
        let mut current_pattern = vec![0; self.num_sites];
        for (((_idx, el), result_val), site) in current_pattern.iter_mut().enumerate().zip(closed_sites.iter()).zip(site_array.iter()) {
            if !result_val { // wenn nicht geschlossen
                *el = u8::min(site.capacity, site_cap)
            }
        }
        current_pattern
    }
}


