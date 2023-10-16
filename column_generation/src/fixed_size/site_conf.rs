












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
}


