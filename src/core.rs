use std::cmp::min;

use dashmap::DashMap;
use std::collections::HashMap;

use crate::utils;

use packed_simd::*;

use std::array::LengthAtMost32; // temporary until its removed

pub type PowerLevel = i16;

lazy_static! {
    pub static ref ALL_MSDS: std::vec::Vec<std::vec::Vec<u8>> = utils::compositions(8, 8);
}


pub struct Configuration<const N: usize> {
    pub powerful_start : PowerLevel,
    pub powerful_cap : PowerLevel,
    pub pinnacle_cap : PowerLevel,

    pub actions : [Action; N],

    pub all_entries: Vec<StateEntry>
}

impl<const N: usize> Configuration<N>
  where [Action; N]: LengthAtMost32 {

    /// Constructs a new configuration object, with states from start..pinnacle_cap and a list of actions.
    pub fn make_config(powerful_start: PowerLevel, powerful_cap: PowerLevel, pinnacle_cap: PowerLevel, actions: [Action;N]) -> Configuration<N> {
        let all_entries: Vec<StateEntry> = {
            let mut entries = Vec::new();
            for msd in ALL_MSDS.iter() {
                for mean in powerful_start..pinnacle_cap {
                    if msd.iter().all(|x| (*x as PowerLevel) + mean <= pinnacle_cap) {
                        let mut msd_arr = [0; 8];
                        for i in 0..msd.len() {
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
            powerful_start: powerful_start,
            powerful_cap: powerful_cap,
            pinnacle_cap: pinnacle_cap,
            actions: actions,
            all_entries: all_entries
        };

        return config;
    }

    pub fn update_state_entry(&self, se: &StateEntry, aa: &ActionArity<N>, slot_idx: usize, act_idx: usize) -> (StateEntry, ActionArity<N>, f32) {
        let action = &self.actions[act_idx];

        // update actions
        let mut new_aa = *aa;
        new_aa[act_idx] -= 1;

        // get full slot table
        
        let mut slots = SlotTable::from_cast(i8x8::from(se.mean_slot_deviation)) + se.mean as PowerLevel;


        let old_slot = slots.extract(slot_idx);
        let new_slot = power_gain(self, &action, old_slot);
        let reward = (new_slot - old_slot) as f32;

        slots = slots.replace(slot_idx, new_slot);

        // flatten and calculate new mean (we don't consider flattening a reward really)

        slots = full_flatten(slots);

        let new_mean = current_level(slots);

        let new_msd = i8x8::from_cast(slots - new_mean);

        let mut new_msd_arr = [0; 8];
        new_msd.write_to_slice_aligned(&mut new_msd_arr);

        let new_se = StateEntry {
            mean: new_mean as u16,
            mean_slot_deviation: new_msd_arr,
        };

        (new_se, new_aa, reward)
    }

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
    pub mean_slot_deviation : [i8; Slot::NumberOfSlots as usize],
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct StateTransition {
    pub next_action : u8,
    pub score : f32
}

/// state entry with action arity, for disk acces
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FullStateEntry<const N: usize>
  where [u8; N]: LengthAtMost32 {
    pub available_actions: [u8; N],
    pub state_entry: StateEntry
}

pub type ActionArity<const N: usize> = [i8; N];
/// full access to the currently computed action
pub type CurrentMDPState = HashMap<StateEntry, StateTransition>;
/// read-only view to the entire table
pub type FullMDPState<const N: usize> = DashMap<ActionArity<N>, CurrentMDPState>;


pub struct Solver<const N: usize> {
    pub config: Configuration<N>,
    pub state: FullMDPState<N>,
}

impl<const N: usize> Solver<N>
  where [Action; N]: LengthAtMost32, ActionArity<N>: LengthAtMost32 {

    /// Select an action from an already generated table.
    pub fn select_action(&self, se: &StateEntry, available_actions: &ActionArity<N>) -> Option<StateTransition> {
        if (se.mean as PowerLevel) >= self.config.pinnacle_cap || available_actions.iter().sum::<i8>() == 0 {
            return None;
        }

        Some(self.state.get(available_actions).and_then(|x| x.get(se).map(|elem| *elem))
            .expect("Assert failed: value requested that was not computed. This isn't critical, but important for debugging.")
        )
    }

    /// Create an action.
    /// Note: this MUST be called in ascending rank or it will runtime error
    pub fn create_action(&self, current_state: &mut CurrentMDPState, se: StateEntry, available_actions: &ActionArity<N>) -> StateTransition  {
        let on_action = |idx| -> StateTransition {
            let action: &Action = &self.config.actions[idx];

            let outcomes = action.pmf.iter().enumerate().filter(|(_i,x)| **x > 0.0)
            .map(|(slot, prob)| {
                let (new_se, new_aa, reward) = self.config.update_state_entry(&se, &available_actions, slot, idx);
                let rest_reward = self.select_action(&new_se, &new_aa).map(|x| x.score).unwrap_or(0.0);
                prob * (reward + rest_reward)
            });
            let value = outcomes.into_iter().sum();
            StateTransition {
                next_action: idx as u8, score: value
            }
        };

        let mut max_st = StateTransition { next_action: 0, score: 0.0 };
        for i in 0..available_actions.len() {
            if available_actions[i] > 0 {
                let new_st = on_action(i);
                if new_st.score >= max_st.score {
                    max_st = new_st;
                }
            }
        }
        current_state.insert(se, max_st); 
        max_st
    }

    /// Bottom-up building of states (as ordered by available actions)
    pub fn build_states(&self, current_state: &mut CurrentMDPState, actions: &ActionArity<N>) 
    where ActionArity<N>: LengthAtMost32, [Action; N]: LengthAtMost32 {
        for se in &self.config.all_entries {
            self.create_action(current_state, se.clone(), actions);
        }
    }

}







pub fn power_gain<const N: usize>(config: &Configuration<N>, action: &Action, old_slot: PowerLevel) -> PowerLevel 
  where [Action; N]: LengthAtMost32 {
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

