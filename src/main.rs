#![feature(const_generics)]

use std::iter::FromIterator;
use smallvec::SmallVec;
use dashmap::DashMap;


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
    let state = DashMap::new();

    for acts in utils::ranked_actions(&cap) {
        for act in &acts {
            build_states(&config, &state, &SmallVec::from_vec(act.to_vec()));
        }
        println!("Actions: {:?}", acts.len());
    }
    println!("State size: {:?}", state.len());
}