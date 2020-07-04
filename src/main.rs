#![feature(const_generics)]

#[macro_use]
extern crate lazy_static;

use std::iter::FromIterator;
use smallvec::SmallVec;
use dashmap::DashMap;

use rayon::prelude::*;


pub mod core;
mod utils;

use crate::core::*;


fn main() {
    let default_pmf = [ 1.0/8.0, 1.0/8.0, 1.0/8.0, 1.0/8.0, 1.0/8.0, 1.0/8.0, 1.0/8.0, 1.0/8.0 ];
    let armor_pmf   = [ 0.0,     0.0,     0.0,     1.0/5.0, 1.0/5.0, 1.0/5.0, 1.0/5.0, 1.0/5.0 ];
    let raid1_pmf   = [ 1.0/3.0, 1.0/3.0, 0.0,     0.0,     0.0,     0.0,     1.0/3.0, 0.0     ];
	let raid2_pmf   = [ 0.0    , 1.0/2.0, 0.0,     0.0,     1.0/2.0, 0.0,     0.0,     0.0     ];
	let raid3_pmf   = [ 1.0/3.0, 1.0/3.0, 0.0,     0.0,     0.0,     1.0/3.0, 0.0,     0.0     ];
	let raid4_pmf   = [ 0.0,     1.0/3.0, 0.0,     0.0,     1.0/3.0, 0.0,     0.0,     0.0     ];


    let config = Configuration {
        powerful_cap: 1050,
        pinnacle_cap: 1060,
        actions: [
            Action { powerful_gain: 5, pinnacle_gain: 1, arity: 4, pmf: default_pmf },
            Action { powerful_gain: 5, pinnacle_gain: 2, arity: 3, pmf: default_pmf },
            Action { powerful_gain: 5, pinnacle_gain: 2, arity: 2, pmf: armor_pmf },

            Action { powerful_gain: 5, pinnacle_gain: 2, arity: 1, pmf: raid1_pmf },
            Action { powerful_gain: 5, pinnacle_gain: 2, arity: 1, pmf: raid2_pmf },
            Action { powerful_gain: 5, pinnacle_gain: 2, arity: 2, pmf: raid3_pmf },
            Action { powerful_gain: 5, pinnacle_gain: 2, arity: 1, pmf: raid4_pmf },
        ]
    };

    let cap = config.actions.iter().map(|x| x.arity ).collect();
    let ranks = utils::ranked_actions(&cap);
    
    let mut progress = 0;
    let total = ranks.iter().map(|x| x.len()).sum::<usize>();

    let state = DashMap::with_capacity(total * 10 * 6435); // 10: power levels, 6435: k-combinations; just quick and dirty..

    for acts in ranks {
        acts.par_iter().for_each(|act| build_states(&config, &state, &SmallVec::from_vec(act.to_vec())));
        progress += acts.len();
        println!("Actions: {}/{}", progress, total);
    }
    println!("State size: {:?}", state.len());
}