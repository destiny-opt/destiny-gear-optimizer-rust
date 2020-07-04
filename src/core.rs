use std::cmp::min;

use dashmap::DashMap;
use std::collections::HashMap;

use crate::utils;

use packed_simd::*;

pub type PowerLevel = i16;

pub struct Configuration {
    pub actions : Vec<Action>,
    pub powerful_start : PowerLevel,
    pub powerful_cap : PowerLevel,
    pub pinnacle_cap : PowerLevel,

    pub all_entries: Vec<StateEntry>
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
pub type SlotTable = i16x8;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct StateEntry {
    pub mean : u16,
    pub mean_slot_deviation : i8x8,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct StateTransition {
    pub next_action : u8,
    pub score : f32
}

pub type ActionArity = i8x16;

/// full access to the currently computed action
pub type CurrentMDPState = HashMap<StateEntry, StateTransition>;
/// read-only view to the entire table
pub type FullMDPState = DashMap<ActionArity, CurrentMDPState>;

/// Select an action from an already generated table.
pub fn select_action(config: &Configuration, full_state: &FullMDPState, se: &StateEntry, available_actions: &ActionArity) -> Option<StateTransition> {
    if (se.mean as PowerLevel) >= config.pinnacle_cap || available_actions.wrapping_sum() == 0 {
        return None;
    }

    Some(full_state.get(available_actions).and_then(|x| x.get(se).map(|elem| *elem))
        .expect("Assert failed: value requested that was not computed. This isn't critical, but important for debugging.")
    )
}

/// Create an action.
/// Note: this MUST be called in ascending rank or it will runtime error
pub fn create_action(config: &Configuration, full_state: &FullMDPState, current_state: &mut CurrentMDPState, se: StateEntry, available_actions: &ActionArity) {
    let on_action = |idx| -> StateTransition {
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

    let mut max_st = StateTransition { next_action: 0, score: 0.0 };
    for i in 0..ActionArity::lanes() {
        if available_actions.extract(i) > 0 {
            let new_st = on_action(i);
            if new_st.score >= max_st.score {
                max_st = new_st;
            }
        }
    }
     current_state.insert(se, max_st); 
}

pub fn update_state_entry(config: &Configuration, se: &StateEntry, aa: &ActionArity, slot_idx: usize, act_idx: usize) -> (StateEntry, ActionArity, f32) {
    let action = &config.actions[act_idx];

    // update actions
    let new_aa = aa.replace(act_idx, aa.extract(act_idx) - 1);

    // get full slot table
    
    let mut slots = SlotTable::from_cast(i8x8::from(se.mean_slot_deviation)) + se.mean as PowerLevel;


    let old_slot = slots.extract(slot_idx);
    let new_slot = power_gain(config, &action, old_slot);
    let reward = (new_slot - old_slot) as f32;

    slots = slots.replace(slot_idx, new_slot);

    // flatten and calculate new mean (we don't consider flattening a reward really)

    slots = full_flatten(slots);

    let new_mean = current_level(slots);

    let new_msd = i8x8::from_cast(slots - new_mean);

    let new_se = StateEntry {
        mean: new_mean as u16,
        mean_slot_deviation: new_msd,
    };

    (new_se, new_aa, reward)
}

pub fn power_gain(config: &Configuration, action: &Action, old_slot: PowerLevel) -> PowerLevel {
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

pub fn current_level(slots: SlotTable) -> PowerLevel {
    slots.wrapping_sum() / (SlotTable::lanes() as PowerLevel)
}

pub fn full_flatten(slots: SlotTable) -> SlotTable {
    let mut slots_mut = slots;
    let mut current = current_level(slots);

    while slots_mut.min_element() < current {
        slots_mut = slots_mut.max(SlotTable::splat(current));
        current = current_level(slots_mut);
    }
    slots_mut
}

// Bottom-up building (as ordered by available actions)

lazy_static! {
    pub static ref ALL_MSDS: std::vec::Vec<std::vec::Vec<u8>> = utils::compositions(8, 8);
}

pub fn build_states(config: &Configuration, full_state: &FullMDPState, current_state: &mut CurrentMDPState, actions: &ActionArity) {
    for se in &config.all_entries {
        create_action(config, full_state, current_state, se.clone(), actions);
    }
}