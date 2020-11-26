#![feature(option_expect_none)]
#![feature(const_generics)]
#![feature(const_generic_impls_guard)]


#[macro_use]
extern crate lazy_static;

use dashmap::DashMap;
use std::collections::HashMap;

use rayon::prelude::*;

pub mod core;
mod utils;

use crate::core::*;

use std::array::LengthAtMost32; // temporary until its removed


fn generate_state<const N: usize>(config: Configuration<N>) -> Solver<N> 
  where ActionArity<N>: LengthAtMost32, [Action; N]: LengthAtMost32 {

    let cap: Vec<u8> = config.actions.iter().map(|x| x.arity ).collect();

    // hacky way to encode all possible raid challenges. this needs to be refactored
    let ranks = utils::ranked_actions(cap.len(), cap.iter().sum(), |i, xs| {
        //if i < 3 { 
            xs[i] < cap[i] 
        //} else {
        //    let (_, raids) = xs.split_at(3);            
        //    xs[i] < 1 || (xs[i] == 1 && raids.iter().filter(|x| **x > 1).count() == 0)
        //}
    });

    
    let mut progress = 0;
    let total = ranks.iter().map(|x| x.len()).sum::<usize>();

    let full_state = DashMap::with_capacity(total); // 10: power levels, 6435: k-combinations; just quick and dirty..

    let mut solver = Solver { config: config, state: full_state };

    let se_len = solver.config.all_entries.len();

    for acts in ranks {
        acts.par_iter().for_each(|act| {
            let mut actarr = [0; N];
            for i in 0..act.len() {
                actarr[i] = act[i] as i8;
            }
            let mut current_state: CurrentMDPState = HashMap::with_capacity(se_len);
            solver.build_states(&mut current_state, &actarr);
            //current_state.shrink_to_fit();
            solver.state.insert(actarr, current_state);
        });
        progress += acts.len();
        println!("Actions: {}/{}", progress, total);
    }
    println!("State size: {:?}", solver.state.len() * solver.config.all_entries.len());

    return solver;

}



fn main() {
    
    let default_pmf = [ 1.0/8.0, 1.0/8.0, 1.0/8.0, 1.0/8.0, 1.0/8.0, 1.0/8.0, 1.0/8.0, 1.0/8.0 ];
    let armor_pmf   = [ 0.0,     0.0,     0.0,     1.0/5.0, 1.0/5.0, 1.0/5.0, 1.0/5.0, 1.0/5.0 ];
    let raid1_pmf   = [ 0.0,     1.0/4.0, 0.0,     0.0,     1.0/4.0, 0.0,     1.0/4.0, 1.0/4.0 ];
    let raid2_pmf   = [ 0.0,     1.0/4.0, 0.0,     0.0,     1.0/4.0, 1.0/4.0, 0.0,     1.0/4.0 ];
    let raid3_pmf   = [ 1.0/4.0, 0.0,     0.0,     0.0,     1.0/4.0, 0.0,     1.0/4.0, 1.0/4.0 ];
    let raid4_pmf   = [ 0.0,     0.0,     1.0/4.0, 1.0/4.0, 0.0,     1.0/4.0, 1.0/4.0, 0.0     ];

    let actions = [
        Action { powerful_gain: 5, pinnacle_gain: 1, arity: 4, pmf: default_pmf },
        Action { powerful_gain: 5, pinnacle_gain: 2, arity: 4, pmf: default_pmf },

        Action { powerful_gain: 5, pinnacle_gain: 2, arity: 1, pmf: raid1_pmf },
        Action { powerful_gain: 5, pinnacle_gain: 2, arity: 1, pmf: raid2_pmf },
        Action { powerful_gain: 5, pinnacle_gain: 2, arity: 1, pmf: raid3_pmf },
        Action { powerful_gain: 5, pinnacle_gain: 2, arity: 1, pmf: raid4_pmf },
    ];

    let config = Configuration::make_config(1247, 1250, 1260, actions);
    let solver = generate_state(config);

    let submap = solver.state.get(&[4,4,1,1,1,1]).unwrap();
    let st = submap.get(&StateEntry {
        mean: 1256,
        mean_slot_deviation: [1,1,2,0,1,0,2,0]
    }).unwrap();

    println!("{:?}", st);
    
}