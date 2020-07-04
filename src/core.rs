use std::cmp::min;
use std::cmp::Ordering;
use std::iter::FromIterator;


use smallvec::SmallVec;
use dashmap::DashMap;

use crate::utils;

pub type PowerLevel = i32;

pub struct Configuration<const N: usize> {
    pub actions : [Action; N],
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
pub struct SlotTable<const N: usize> { 
    vec: SmallVec<[i32; N]> 
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StateEntry<const N: usize> {
    pub mean : u16,
    pub mean_slot_deviation : SmallVec<[i8; Slot::NumberOfSlots as usize]>,
    pub available_actions : SmallVec<[u8; N]>
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct StateTransition {
    pub next_action : u8,
    pub score : f32
}

pub type MDPState<const N: usize> = DashMap<StateEntry<N>, StateTransition>;


pub fn select_action<const N: usize>(config: &Configuration<N>, state: &MDPState<N>, se: StateEntry<N>) -> Option<StateTransition> {
    if (se.mean as i32) > config.pinnacle_cap || se.available_actions.iter().sum::<u8>() == 0 {
        return None;
    }

    let on_action = |args| -> StateTransition {
        let (idx, _arity) = args;
        let action: &Action = &config.actions[idx];

        let outcomes = action.pmf.iter().enumerate().filter(|(_i,x)| **x > 0.0)
        .map(|(slot, prob)| {
            let (new_se, reward) = update_state_entry(config, &se, slot, idx);
            let rest_reward = select_action(config, state, new_se).map(|x| x.score).unwrap_or(0.0);
            prob * (reward + rest_reward)
        });
        let value = outcomes.into_iter().sum();
        StateTransition {
            next_action: idx as u8, score: value
        }
    };

    match state.get(&se) {
        Some(st) => Some(*st),
        None => {
            let st = se.available_actions.iter().enumerate()
                .filter(|(_i,x)| **x > 0)
                .map(on_action)
                .max_by(|x, y| x.score.partial_cmp(&y.score).unwrap_or(Ordering::Equal));
            match st {
                Some(st) => { state.insert(se, st); },
                None => {}
            };
            st
        }
    }
}

pub fn update_state_entry<const N: usize>(config: &Configuration<N>, se: &StateEntry<N>, slot_idx: usize, act_idx: usize) -> (StateEntry<N>, f32) {
    let action = &config.actions[act_idx];

    // update actions
    let mut new_aa = se.available_actions.clone();
    new_aa[act_idx] -= 1;

    // get full slot table
    let mut slots = SlotTable { vec: SmallVec::from_iter(se.mean_slot_deviation.iter()
        .map(|x| *x as i32 + se.mean as i32)) 
    };

    let old_slot = slots.vec[slot_idx];
    let new_slot = power_gain(config, &action, old_slot);
    let reward = (new_slot - old_slot) as f32;

    slots.vec[slot_idx] = new_slot;

    // flatten and calculate new mean (we don't consider flattening a reward really)

    full_flatten::<N>(&mut slots);

    let new_mean = current_level(&slots);

    let new_msd = SmallVec::from_iter(
        slots.vec.into_iter().map(|x| (x - new_mean) as i8)
    );

    let new_se = StateEntry {
        mean: new_mean as u16,
        mean_slot_deviation: new_msd,
        available_actions: new_aa
    };

    (new_se, reward)
}

pub fn power_gain<const N: usize>(config: &Configuration<N>, action: &Action, old_slot: i32) -> i32 {
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

pub fn current_level<const N: usize>(slots: &SlotTable<N>) -> i32 {
    slots.vec.iter().sum::<i32>() / (slots.vec.len() as i32)
}

// returns true if flattening did something
pub fn flatten<const N: usize>(slots: &mut SlotTable<N>) -> bool {
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

pub fn full_flatten<const N: usize>(slots: &mut SlotTable<N>) {
    while flatten(slots) {
        // blank (this terminates, i swear)
    }
}





// Bottom-up building (as ordered by available actions)

lazy_static! {
    static ref ALL_MSDS: std::vec::Vec<std::vec::Vec<u8>> = utils::compositions(8, 8);
}

pub fn build_states<const N: usize>(config: &Configuration<N>, state: &MDPState<N>, actions: &SmallVec<[u8; N]>) {
    for msd in ALL_MSDS.iter() {
        for mean in config.powerful_cap..=config.pinnacle_cap-1 {
            if msd.iter().all(|x| (*x as PowerLevel) + mean < config.pinnacle_cap) {
                let se = StateEntry { 
                    mean: mean as u16,
                    mean_slot_deviation: SmallVec::from_iter(msd.iter().map(|x| *x as i8)),
                    available_actions: actions.clone()
                };
                select_action(config, state, se);
            }
        }
    }
}