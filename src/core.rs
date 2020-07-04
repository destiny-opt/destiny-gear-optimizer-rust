use std::cmp::min;
use std::cmp::Ordering;

use smallvec::SmallVec;
use dashmap::{DashMap, ReadOnlyView};
use std::collections::HashMap;

use crate::utils;

pub type PowerLevel = i32;

pub struct Configuration {
    pub actions : Vec<Action>,
    pub powerful_cap : PowerLevel,
    pub pinnacle_cap : PowerLevel
}

#[derive(Debug, Clone,  PartialEq)]
pub struct Action {
    pub pinnacle_gain : PowerLevel,
    pub powerful_gain : PowerLevel,
    pub arity: u8,
    pub pmf : [f32; Slot::NumberOfSlots as usize] // TODO: make this safe (require sum=1)
}

#[repr(u8)]
#[derive(Debug, Copy, Clone,  PartialEq,  PartialOrd)]
pub enum Slot {
    Kinetic = 0,
    Energy,
    Power,
    Head,
    Glove,
    Chest,
    Leg,
    ClassItem,
    NumberOfSlots
}

// general purpose slot table
pub struct SlotTable { 
    vec: [i32; 8]
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StateEntry {
    pub mean : u16,
    pub mean_slot_deviation : [i8; Slot::NumberOfSlots as usize],
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct StateTransition {
    pub next_action : u8,
    pub score : f32
}

pub type ActionArity = SmallVec<[u8; 16]>;

/// full access to the currently computed action
pub type CurrentMDPState = HashMap<StateEntry, StateTransition>;
/// read-only view to the entire table
pub type FullMDPState = DashMap<ActionArity, CurrentMDPState>;

/// Select an action from an already generated table.
pub fn select_action(config: &Configuration, full_state: &FullMDPState, se: &StateEntry, available_actions: &ActionArity) -> Option<StateTransition> {
    if (se.mean as i32) >= config.pinnacle_cap || available_actions.iter().sum::<u8>() == 0 {
        return None;
    }

    Some(full_state.get(available_actions).and_then(|x| x.get(se).map(|elem| *elem))
        .expect(&format!("We technically support {:?}/{:?} having no entry, but it really shouldn't be", available_actions, se))
    )
}

/// Create an action.
/// Note: this MUST be called in ascending rank or it will runtime error
pub fn create_action(config: &Configuration, full_state: &FullMDPState, current_state: &mut CurrentMDPState, se: StateEntry, available_actions: &ActionArity) {
    let on_action = |args| -> StateTransition {
        let (idx, _arity) = args;
        let action: &Action = &config.actions[idx];

        let outcomes = action.pmf.iter().enumerate().filter(|(_i,x)| **x > 0.0)
        .map(|(slot, prob)| {
            let (new_se, new_aa, reward) = update_state_entry(config, &se, &available_actions, slot, idx);
            let rest_reward = select_action(config, full_state, &new_se, &new_aa).map(|x| x.score).unwrap_or(0.0);
            prob * (reward + rest_reward)
        });
        let value = outcomes.into_iter().sum();
        StateTransition {
            next_action: idx as u8, score: value
        }
    };

    let st = available_actions.iter().enumerate()
        .filter(|(_i,x)| **x > 0)
        .map(on_action)
        .max_by(|x, y| x.score.partial_cmp(&y.score).unwrap_or(Ordering::Equal));
    match st {
        Some(st) => { current_state.insert(se, st); },
        None => {}
    };
    

    
}

pub fn update_state_entry(config: &Configuration, se: &StateEntry, aa: &ActionArity, slot_idx: usize, act_idx: usize) -> (StateEntry, ActionArity, f32) {
    let action = &config.actions[act_idx];

    // update actions
    let mut new_aa = aa.clone();
    new_aa[act_idx] -= 1;

    // get full slot table
    
    let mut slots = SlotTable { vec: [0; 8] };

    for i in 0..slots.vec.len() {
        slots.vec[i] = se.mean_slot_deviation[i] as i32 + se.mean as i32;
    }

    let old_slot = slots.vec[slot_idx];
    let new_slot = power_gain(config, &action, old_slot);
    let reward = (new_slot - old_slot) as f32;

    slots.vec[slot_idx] = new_slot;

    // flatten and calculate new mean (we don't consider flattening a reward really)

    full_flatten(&mut slots);

    let new_mean = current_level(&slots);

    let mut new_msd = [0; 8];
    for i in 0..new_msd.len() {
        new_msd[i] = (slots.vec[i] - new_mean) as i8;
    }

    let new_se = StateEntry {
        mean: new_mean as u16,
        mean_slot_deviation: new_msd,
    };

    (new_se, new_aa, reward)
}

pub fn power_gain(config: &Configuration, action: &Action, old_slot: i32) -> i32 {
    if old_slot < config.powerful_cap {
        // this models behavior at the transition region (e.g. +2 pinnacle at 1047 light gives 1052, +1 or +2 pinnacle at 1046 gives 1051, +2 at 1049 gives 1052)
        min(old_slot + action.powerful_gain, config.powerful_cap + action.pinnacle_gain)
    } else if old_slot < config.pinnacle_cap {
        min(old_slot + action.pinnacle_gain, config.pinnacle_cap)
    } else {
        assert!(old_slot <= config.pinnacle_cap, "old slot {:?} exceeds pinnacle cap {:?}! check your logic.", old_slot, config.pinnacle_cap);
        // we're at pinnacle cap
        config.pinnacle_cap
    }
}

pub fn current_level(slots: &SlotTable) -> i32 {
    slots.vec.iter().sum::<i32>() / (slots.vec.len() as i32)
}

// returns true if flattening did something
pub fn flatten(slots: &mut SlotTable) -> bool {
    let current = current_level(slots);
    let mut success = false;
    for x in slots.vec.iter_mut() {
        if *x < current {
            success = true;
            *x = current;
        }
    }
    return success;
}

pub fn full_flatten(slots: &mut SlotTable) {
    while flatten(slots) {
        // blank (this terminates, i swear)
    }
}





// Bottom-up building (as ordered by available actions)

lazy_static! {
    static ref ALL_MSDS: std::vec::Vec<std::vec::Vec<u8>> = utils::compositions(8, 8);
}

pub fn build_states(config: &Configuration, full_state: &FullMDPState, current_state: &mut CurrentMDPState, actions: &SmallVec<[u8; 16]>) {
    for msd in ALL_MSDS.iter() {
        for mean in config.powerful_cap..config.pinnacle_cap {
            if msd.iter().all(|x| (*x as PowerLevel) + mean <= config.pinnacle_cap) {
                let mut msd_arr = [0; 8];
                for i in 0..msd_arr.len() {
                    msd_arr[i] = msd[i] as i8;
                }
                let se = StateEntry { 
                    mean: mean as u16,
                    mean_slot_deviation: msd_arr
                };
                create_action(config, full_state, current_state, se, actions);
            }
        }
    }
}