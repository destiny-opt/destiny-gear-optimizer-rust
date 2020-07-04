#![feature(const_generics)]

#[macro_use]
extern crate lazy_static;

use smallvec::SmallVec;
use dashmap::DashMap;
use std::collections::HashMap;

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
    
    let powerful_cap = 1050;
    let pinnacle_cap = 1060;

    let all_entries: Vec<StateEntry> = {
        let mut entries = Vec::new();
        for msd in ALL_MSDS.iter() {
            for mean in powerful_cap..pinnacle_cap {
                if msd.iter().all(|x| (*x as PowerLevel) + mean <= pinnacle_cap) {
                    let mut msd_arr = [0; 8];
                    for i in 0..msd_arr.len() {
                        msd_arr[i] = msd[i] as i8;
                    }
                    entries.push(StateEntry { 
                        mean: mean as u16,
                        mean_slot_deviation: msd_arr
                    })
                }
            }
        }
        entries
    };

    let config = Configuration {
        powerful_cap: powerful_cap,
        pinnacle_cap: pinnacle_cap,
        actions: vec![
            Action { powerful_gain: 5, pinnacle_gain: 1, arity: 4, pmf: default_pmf },
            Action { powerful_gain: 5, pinnacle_gain: 2, arity: 3, pmf: default_pmf },
            Action { powerful_gain: 5, pinnacle_gain: 2, arity: 2, pmf: armor_pmf },

            Action { powerful_gain: 5, pinnacle_gain: 2, arity: 1, pmf: raid1_pmf },
            Action { powerful_gain: 5, pinnacle_gain: 2, arity: 1, pmf: raid2_pmf },
            Action { powerful_gain: 5, pinnacle_gain: 2, arity: 2, pmf: raid3_pmf },
            Action { powerful_gain: 5, pinnacle_gain: 2, arity: 1, pmf: raid4_pmf },
        ],
        all_entries: all_entries
    };

    let cap = config.actions.iter().map(|x| x.arity ).collect();
    let ranks = utils::ranked_actions(&cap);
    
    let mut progress = 0;
    let total = ranks.iter().map(|x| x.len()).sum::<usize>();

    let full_state = DashMap::with_capacity(total); // 10: power levels, 6435: k-combinations; just quick and dirty..


    let se_len = config.all_entries.len();

    for acts in ranks {
        acts.par_iter().for_each(|act| {
            let mut actarr = ActionArity::splat(0);
            for i in 0..act.len() {
                actarr = actarr.replace(i, act[i] as i8);
            }
            let mut current_state: CurrentMDPState = HashMap::with_capacity(se_len);
            build_states(&config, &full_state, &mut current_state, &actarr);
            //current_state.shrink_to_fit();
            full_state.insert(actarr, current_state);
        });
        progress += acts.len();
        println!("Actions: {}/{}", progress, total);
    }
    println!("State size: {:?}", full_state.len() * config.all_entries.len());
}