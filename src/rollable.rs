use rltk::RandomNumberGenerator;
use specs::prelude::*;

pub trait Rollable {
    fn roll(&mut self, n: i32, die_type: i32) -> i32;
}

impl Rollable for World {
    fn roll(&mut self, n: i32, die_type: i32) -> i32 {
        let mut rng = self.write_resource::<RandomNumberGenerator>();
        rng.roll_dice(n, die_type)
    }
}
