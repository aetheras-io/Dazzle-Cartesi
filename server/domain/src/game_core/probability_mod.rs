use super::lazy_static;
use rand::distributions::{Distribution, Uniform};
use std::ops::{Bound, RangeBounds};
use std::sync::{Arc, RwLock};

use crate::game_core::config::TUTORIAL_RIVAL_ADDR;

pub const LOCK_POISONED: &str = "Lock is poisoned";

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum ProbGroup {
    // Character accessories related
    HP_head_face_neck,
    DEF_body_waist,
    DEF_arm,
    DEF_foot,
    ATK_weapon,
    ATK_weapon_in_top_rarity,
    ATK_sidearms,
    MONO_SPC_FI,      // Floating item
    MONO_SPC_BE,      // Background effect
    DUAL_SPC_FI_SAME, // Two same color
    DUAL_SPC_FI_DIFF, // Two differnt color
    DUAL_SPC_GI_SAME, // Ground item
    DUAL_SPC_GI_DIFF,
    DUAL_SPC_BE,
    DUAL_SPC_GE, // Ground effect

    // Character attribute related
    PASSIVE(usize),
    MONO_SPC_TILE(usize),
    DUAL_SPC_TILE(usize),

    // Other
    ACQUIRED_NEW_CHAR,
}

// Probability threshold to get item in top rarity
pub const P_ACC_TOP_RARITY_TH: u32 = 66;
pub const P_ACC_TOP_RARITY_WEIGHT_RANGE: u32 = 100;

// Probability threshold to get [1, 2, 3] accessories
pub const P_HP_ACC_TH_LIST: &[u32] = &[10000, 2500, 125];
pub const P_HP_ACC_WEIGHT_RANGE: u32 = 10000;

// Probability threshold to get [1, 2] accessories
pub const P_DEF_ACC_TH_LIST: &[u32] = &[100, 25];
pub const P_DEF_ACC_GET_ARM_TH: u32 = 5;
pub const P_DEF_ACC_GET_FOOT_TH: u32 = 5;
pub const P_DEF_ACC_WEIGHT_RANGE: u32 = 100;

pub const P_ATK_WEAPON_IN_TOP_RARITY_TH: u32 = 33;
pub const P_ATK_ACC_GET_WEAPON_TH: u32 = 99;
pub const P_ATK_ACC_GET_SIDEARMS_TH: u32 = 2;
pub const P_ATK_ACC_WEIGHT_RANGE: u32 = 100;

pub const MONO_SPC_PREM_THRESHOLD: u32 = 120;
pub const P_MONO_SPC_PREM_BG_EFFECT_TH: u32 = 25;
pub const P_MONO_SPC_PREM_BG_EFFECT_WEIGHT_RANGE: u32 = 1000;

pub const P_DUAL_SPC_FI_SAME_TH: u32 = 25;
pub const P_DUAL_SPC_FI_DIFF_TH: u32 = 990;
pub const P_DUAL_SPC_GI_SAME_TH: u32 = 990;
pub const P_DUAL_SPC_GI_DIFF_TH: u32 = 25;
pub const P_DUAL_SPC_BG_EFFECT_TH: u32 = 25;
pub const P_DUAL_SPC_WEIGHT_RANGE: u32 = 1000;

// NFT tier related probability - Tier[0, 1, 2, 3]
pub const P_PASSIVE_TH: &[u32] = &[100, 99, 50, 50, 100];
pub const P_MONO_SPC_TILE_TH: &[u32] = &[100, 100, 50, 5, 100];
pub const P_DUAL_SPC_TILE_TH: &[u32] = &[5, 1, 0, 0, 100];
pub const P_ATTRIBUTE_WEIGHT_RANGE: u32 = 100;

// Temp for testing, 100% guaranteed to acquired.
pub const P_ACQUIRE_NEW_CHARACTER_TH: u32 = 100;
pub const P_ACQUIRE_NEW_CHARACTER_WEIGHT_RANGE: u32 = 100;

#[derive(Debug, Clone, Default)]
pub struct RandomNumHolder {
    pub bitmask_rand_pool: u128,
    pub valid_bit: u32,
    pub bit_consumed: u32,  // for debug
    pub rand_consumed: u32, // for debug
}

lazy_static::lazy_static! {
    pub static ref RANDOM_NUM_HOLDER: Arc<RwLock<RandomNumHolder>> = Arc::new(RwLock::new(RandomNumHolder::new(0)));
}

impl RandomNumHolder {
    pub fn new(consumed: u32) -> RandomNumHolder {
        let mut rng = rand::thread_rng();
        Self {
            bitmask_rand_pool: Uniform::new(0, std::u128::MAX).sample(&mut rng),
            valid_bit: u128::BITS,
            bit_consumed: 0,
            rand_consumed: consumed,
        }
    }

    fn generate_new_rand_pool(&mut self) {
        *self = RandomNumHolder::new(self.rand_consumed);
        self.rand_consumed += 1;
    }

    /// Sample a value in `range`
    pub fn sample(&mut self, range: impl RangeBounds<u32>) -> u32 {
        let start = match range.start_bound() {
            Bound::Included(&s) => s,
            Bound::Excluded(&s) => s + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&e) => e,
            Bound::Excluded(&e) => e - 1,
            Bound::Unbounded => std::u32::MAX,
        };

        // In practice, caller shold not make start == end, but still need to handle this case here.
        let consume_bit = u128::BITS - ((end - start + 1) as u128).leading_zeros();
        if self.valid_bit < consume_bit {
            self.generate_new_rand_pool();
        }
        let result = self.bitmask_rand_pool % ((end - start + 1) as u128);
        self.bitmask_rand_pool = self.bitmask_rand_pool >> consume_bit;
        self.valid_bit -= consume_bit;
        self.bit_consumed += consume_bit;
        result as u32 + start
    }
}

/// Decide how many items can be acquired
pub fn roll_possess_amount(p_group: ProbGroup) -> usize {
    let (threshold_list, weight_range) = match p_group {
        ProbGroup::HP_head_face_neck => (P_HP_ACC_TH_LIST, P_HP_ACC_WEIGHT_RANGE),
        ProbGroup::DEF_body_waist => (P_DEF_ACC_TH_LIST, P_DEF_ACC_WEIGHT_RANGE),
        _ => unreachable!(),
    };

    let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
    let rand = rand_holder.sample(..weight_range);
    let mut acquired_amount = 0;
    for acquire_threshold in threshold_list {
        if rand < *acquire_threshold {
            acquired_amount += 1;
        }
    }
    acquired_amount
}

/// Decide whether a single item can be acquired
pub fn roll_possess(p_group: ProbGroup) -> bool {
    let (acquire_threshold, weight_range) = match p_group {
        ProbGroup::DEF_arm => (P_DEF_ACC_GET_ARM_TH, P_DEF_ACC_WEIGHT_RANGE),
        ProbGroup::DEF_foot => (P_DEF_ACC_GET_FOOT_TH, P_DEF_ACC_WEIGHT_RANGE),
        ProbGroup::ATK_weapon => (P_ATK_ACC_GET_WEAPON_TH, P_ATK_ACC_WEIGHT_RANGE),
        ProbGroup::ATK_weapon_in_top_rarity => {
            (P_ATK_WEAPON_IN_TOP_RARITY_TH, P_ATK_ACC_WEIGHT_RANGE)
        }
        ProbGroup::ATK_sidearms => (P_ATK_ACC_GET_SIDEARMS_TH, P_ATK_ACC_WEIGHT_RANGE),
        ProbGroup::MONO_SPC_FI => (P_ACC_TOP_RARITY_TH, P_ACC_TOP_RARITY_WEIGHT_RANGE),
        ProbGroup::MONO_SPC_BE => (
            P_MONO_SPC_PREM_BG_EFFECT_TH,
            P_MONO_SPC_PREM_BG_EFFECT_WEIGHT_RANGE,
        ),
        ProbGroup::DUAL_SPC_GE => (P_ACC_TOP_RARITY_TH, P_ACC_TOP_RARITY_WEIGHT_RANGE),
        ProbGroup::DUAL_SPC_FI_SAME => (P_DUAL_SPC_FI_SAME_TH, P_DUAL_SPC_WEIGHT_RANGE),
        ProbGroup::DUAL_SPC_FI_DIFF => (P_DUAL_SPC_FI_DIFF_TH, P_DUAL_SPC_WEIGHT_RANGE),
        ProbGroup::DUAL_SPC_GI_SAME => (P_DUAL_SPC_GI_SAME_TH, P_DUAL_SPC_WEIGHT_RANGE),
        ProbGroup::DUAL_SPC_GI_DIFF => (P_DUAL_SPC_GI_DIFF_TH, P_DUAL_SPC_WEIGHT_RANGE),
        ProbGroup::DUAL_SPC_BE => (P_DUAL_SPC_BG_EFFECT_TH, P_DUAL_SPC_WEIGHT_RANGE),
        ProbGroup::ACQUIRED_NEW_CHAR => (
            P_ACQUIRE_NEW_CHARACTER_TH,
            P_ACQUIRE_NEW_CHARACTER_WEIGHT_RANGE,
        ),
        ProbGroup::PASSIVE(tier_lv) => (P_PASSIVE_TH[tier_lv], P_ATTRIBUTE_WEIGHT_RANGE),
        ProbGroup::MONO_SPC_TILE(tier_lv) => {
            (P_MONO_SPC_TILE_TH[tier_lv], P_ATTRIBUTE_WEIGHT_RANGE)
        }
        ProbGroup::DUAL_SPC_TILE(tier_lv) => {
            (P_DUAL_SPC_TILE_TH[tier_lv], P_ATTRIBUTE_WEIGHT_RANGE)
        }
        _ => unreachable!(),
    };
    let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
    let rand = rand_holder.sample(..weight_range);
    rand < acquire_threshold
}

pub fn is_new_character_get(winner_id: &str) -> bool {
    if winner_id == TUTORIAL_RIVAL_ADDR {
        return false;
    }
    // ### TODO: No rule for now, using a fixed probability.
    // Related issue: #465
    roll_possess(ProbGroup::ACQUIRED_NEW_CHAR)
}
