use crate::fixed_size::brancher::Brancher;

pub struct ScenarioManager<'a> {
    pub branchers : Vec<Brancher<'a>>,
    pub active_sets : Vec<bool>,
    pub generation_set : Vec<bool>

}

impl <'a> ScenarioManager<'a> {

    pub fn new(branchers : Vec<Brancher<'a>>) -> Self {

        let active_sets = vec![false;branchers.len()];
        let generation_set = vec![false;branchers.len()];
        ScenarioManager{
            branchers,
            active_sets,
            generation_set
        }
    }

    pub fn add_brancher_and_activate(&mut self, brancher : Brancher<'a>) {
        self.branchers.push(brancher);
        self.active_sets.push(true);
        self.generation_set.push(true);
    }

    pub fn new_generation(&mut self) {
        self.generation_set = vec![false;self.branchers.len()];
    }


    pub fn get_all_branchers(&mut self) -> impl Iterator<Item =  (usize,&mut Brancher<'a>)>{
        self.branchers.iter_mut().enumerate()
    }

    pub fn get_active_branchers(&mut self) -> impl Iterator<Item = (usize,&mut Brancher<'a>)>{
        self.active_sets.iter().zip(self.branchers.iter_mut().enumerate())
            .filter_map(|(f, b) | {
                if *f == true {
                    Some(b)
                } else {
                    None
                }
            })
    }


    pub fn get_inactive_branchers(&mut self) -> impl Iterator<Item =  (usize,&mut Brancher<'a>)>{
        self.active_sets.iter().zip(self.branchers.iter_mut().enumerate())
            .filter_map(|(f, b) | {
                if *f == false {
                    Some(b)
                } else {
                    None
                }
            })

    }



    pub fn activate(&mut self, index : usize) {
        self.active_sets[index] = true;
        self.generation_set[index] =true;
    }

    pub fn num_active(&self) -> usize {
        self.active_sets.iter().filter(|x| **x).count()
    }

    pub fn deactivate(&mut self, index : usize) {
        self.active_sets[index] = false;
        self.generation_set[index] = true;
    }
}