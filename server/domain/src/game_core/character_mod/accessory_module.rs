//use atb::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp;
use strum::IntoEnumIterator;
use strum_macros::{EnumCount as EnumCountMacro, EnumIter};

use super::art_assets_count::{AccPartFileName, ART_ASSET_AMOUNT};
use super::attribute::{Attribute, SpecialTile};
use super::char_const::*;
use crate::game_core::config::{Element, GameplayConfigManager};
use crate::game_core::probability_mod::*;

#[derive(
    Debug, Copy, Clone, Deserialize, Serialize, EnumCountMacro, Eq, EnumIter, PartialEq, Hash,
)]
pub enum AccPart {
    // HP accessory
    Head,
    Face,
    Neck,
    // DEF accessory
    Body,
    Waist,
    Arm,
    Foot,
    // ATK accessoy
    Eyes,
    Weapon,
    Sidearms,
    // MONO SPC accessory
    FloatingItem1,
    GroundItem1,
    BackgroundEffect1,
    // DUAL SPC accessory
    GroundEffect,
    FloatingItem2,
    GroundItem2,
    BackgroundEffect2, // Should be ignored if BackgroundEffect1 not empty
}

#[derive(Debug, Clone, Deserialize, Serialize, EnumCountMacro, PartialEq)]
pub enum PrimitiveEyes {
    Origin = 10,
    PecoraEyes = 11,
    AvesEyes = 12,
    FelidaeEyes = 13,
    CanidaeEyes = 14,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AccItemLv {
    lv: u8,
    item_index: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AccessoryModule {
    pub accessory_list: Vec<u32>, // Index corresponding to enum AccPart
}

impl AccessoryModule {
    pub fn roll_accessory(attribute: &Attribute, config: &GameplayConfigManager) -> Self {
        let mut accessory_list = vec![];

        accessory_list.extend(Self::roll_hp_accessory(attribute.get_max_hp(), config));
        accessory_list.extend(Self::roll_def_accessory(attribute.get_def(), config));
        accessory_list.extend(Self::roll_atk_accessory(attribute.get_atk(), config));
        accessory_list.extend(Self::roll_mono_spc_accessory(
            attribute.get_special_tile(),
            config,
        ));
        accessory_list.extend(Self::roll_dual_spc_accessory(
            attribute.get_special_tile(),
            config,
        ));

        /*
        let rand_holder = RANDOM_NUM_HOLDER.read().expect(LOCK_POISONED);
        log::debug!(
            "   ### Accessories used_bit:{}, rand_consumed: {}",
            rand_holder.bit_consumed,
            rand_holder.rand_consumed
        );
        */

        Self { accessory_list }
    }

    /// Decide (Head, Face, Neck)
    fn roll_hp_accessory(hp: u32, config: &GameplayConfigManager) -> Vec<u32> {
        let attr_config = config.get_char_attr_config();
        let rarity_lv_cap = Self::get_rarity_lv_cap(
            hp,
            attr_config.hp_min,
            attr_config.hp_max - attr_config.hp_min,
        );

        // Init premium part pool
        let mut remain_pool = vec![
            AccPart::Head as usize,
            AccPart::Face as usize,
            AccPart::Neck as usize,
        ];

        let pick_num = roll_possess_amount(ProbGroup::HP_head_face_neck);
        let result_acc_list = Self::pick_accessories(pick_num, rarity_lv_cap, &mut remain_pool);

        // [Head, Face, Neck]
        result_acc_list
    }

    /// Decide (Body, Waist, Arm, Foot)
    fn roll_def_accessory(def: u32, config: &GameplayConfigManager) -> Vec<u32> {
        let attr_config = config.get_char_attr_config();
        let rarity_lv_cap = Self::get_rarity_lv_cap(
            def,
            attr_config.def_min,
            attr_config.def_max - attr_config.def_min,
        );

        // Roll Body & waist accessory
        // Init premium part pool
        let mut remain_pool = vec![AccPart::Body as usize, AccPart::Waist as usize];

        let pick_num = roll_possess_amount(ProbGroup::DEF_body_waist);
        let mut result_acc_list = Self::pick_accessories(pick_num, rarity_lv_cap, &mut remain_pool);

        // Roll Arm accessory
        result_acc_list.push(if roll_possess(ProbGroup::DEF_arm) {
            let lv = Self::roll_lv(1, rarity_lv_cap);
            Self::compose_to_byte_array(lv, Self::roll_item_index(AccPartFileName::arm, lv))
        } else {
            0
        });

        // Roll Foot Accessory
        result_acc_list.push(if roll_possess(ProbGroup::DEF_foot) {
            let lv = Self::roll_lv(1, rarity_lv_cap);
            Self::compose_to_byte_array(lv, Self::roll_item_index(AccPartFileName::foot, lv))
        } else {
            0
        });

        // [Body, Waist, Arm, Foot]
        result_acc_list
    }

    /// Decide (Eyes, Weapon, Sidearms)
    fn roll_atk_accessory(atk: u32, config: &GameplayConfigManager) -> Vec<u32> {
        let attr_config = config.get_char_attr_config();
        let rarity_lv_cap = Self::get_rarity_lv_cap(
            atk,
            attr_config.atk_min,
            attr_config.atk_max - attr_config.atk_min,
        );
        let mut result_acc_list = vec![];

        // Roll eyes
        result_acc_list.push(if rarity_lv_cap < 3 {
            // Use primitive race eyes
            // Each race's primitive eyes has only 1 item.
            Self::compose_to_byte_array(PrimitiveEyes::Origin as usize, 1)
        } else {
            // Roll high lv eyes
            let lv = Self::roll_eyes(rarity_lv_cap);
            Self::compose_to_byte_array(
                lv, // If rolled eyes lv large than 10, ignore item index field (using primitive race eyes)
                Self::roll_item_index(AccPartFileName::eye, lv),
            )
        });

        // Roll weapon
        result_acc_list.push(if roll_possess(ProbGroup::ATK_weapon) {
            // Top rarity weapon has only 33% chance to acquire
            let weapon_lv = if roll_possess(ProbGroup::ATK_weapon_in_top_rarity) {
                rarity_lv_cap
            } else {
                Self::roll_lv(1, rarity_lv_cap - 1)
            };

            Self::compose_to_byte_array(
                weapon_lv,
                Self::roll_item_index(AccPartFileName::weapon, weapon_lv),
            )
        } else {
            0
        });

        // Roll sidearms
        result_acc_list.push(if roll_possess(ProbGroup::ATK_sidearms) {
            let lv = Self::roll_lv(1, rarity_lv_cap);
            Self::compose_to_byte_array(lv, Self::roll_item_index(AccPartFileName::sidearms, lv))
        } else {
            0
        });

        // [Eyes, Weapon, Sidearms]
        result_acc_list
    }

    /// Decide (Floating item 1, Ground item 1, Background effect)
    fn roll_mono_spc_accessory(
        special_tile: &SpecialTile,
        config: &GameplayConfigManager,
    ) -> Vec<u32> {
        let mut result_acc_list = vec![0, 0, 0];
        let element = special_tile.element1;
        let elem_boost = special_tile.elem1_boost_val;

        // No special tile
        if element == Element::Unknown {
            return result_acc_list;
        }

        let attr_config = config.get_char_attr_config();
        let rarity_lv_cap = Self::get_rarity_lv_cap(
            elem_boost,
            attr_config.mono_sp_gem_min,
            attr_config.mono_sp_gem_max - attr_config.mono_sp_gem_min,
        );

        // Roll floating item
        let floatinf_item_lv = if roll_possess(ProbGroup::MONO_SPC_FI) {
            // Top rarity item has only 66% chance to accquare
            rarity_lv_cap
        } else {
            // Not top rarity items will rolled evenly
            Self::roll_lv(1, rarity_lv_cap - 1)
        };
        let enum_offset = AccPart::FloatingItem1 as usize;
        result_acc_list[AccPart::FloatingItem1 as usize - enum_offset] =
            Self::compose_to_byte_array(
                floatinf_item_lv,
                Self::roll_item_index(AccPartFileName::floatingItem, floatinf_item_lv),
            );

        // Roll ground item & bg effect, only special tile boost val >= 120 has chance to roll
        if elem_boost > MONO_SPC_PREM_THRESHOLD {
            // Roll ground item, guaranteed to acquire one ground item and with top rarity
            result_acc_list[AccPart::GroundItem1 as usize - enum_offset] =
                Self::compose_to_byte_array(
                    rarity_lv_cap,
                    Self::roll_item_index(AccPartFileName::groundItem, rarity_lv_cap),
                );

            // Roll bg effect
            if roll_possess(ProbGroup::MONO_SPC_BE) {
                let lv = Self::roll_lv(1, MAX_RARITY_LV);
                // Backgound can acquire all rairity lv from pool
                result_acc_list[AccPart::BackgroundEffect1 as usize - enum_offset] =
                    Self::compose_to_byte_array(
                        lv,
                        Self::roll_item_index(AccPartFileName::backgroundEffect, lv),
                    )
            };
        };

        // [FloatingItem1, GroundItem1, BackgroundEffect1]
        result_acc_list
    }

    /// Decide (Ground effect, Floating item 2, ground item 2, Background effect 2)
    fn roll_dual_spc_accessory(
        special_tile: &SpecialTile,
        config: &GameplayConfigManager,
    ) -> Vec<u32> {
        let mut result_acc_list = vec![0; 4];

        let element1 = special_tile.element1;
        let element2 = special_tile.element2;
        let val_elem2_boost = special_tile.elem2_boost_val;

        // No special tiles
        if element1 == Element::Unknown || element2 == Element::Unknown {
            return result_acc_list;
        }

        let enum_offset = AccPart::GroundEffect as usize;
        let rarity_lv_cap = Self::get_dual_spc_rarity_lv_cap(val_elem2_boost, config);

        // Roll ground effect. This is 100% guarenteed to acquired so only roll the rarity lv
        let ground_effect_lv = if roll_possess(ProbGroup::DUAL_SPC_GE) {
            // Top rarity item has only 66% chance to accquare
            rarity_lv_cap
        } else {
            Self::roll_lv(1, rarity_lv_cap - 1)
        };
        result_acc_list[AccPart::GroundEffect as usize - enum_offset] = Self::compose_to_byte_array(
            ground_effect_lv,
            Self::roll_item_index(AccPartFileName::groundEffect, ground_effect_lv),
        );

        // Roll floating item, mono or dual elements has different chance
        let p_group_fi = if element1 == element2 {
            ProbGroup::DUAL_SPC_FI_SAME
        } else {
            ProbGroup::DUAL_SPC_FI_DIFF
        };
        if roll_possess(p_group_fi) {
            let lv = Self::roll_lv(1, rarity_lv_cap);
            result_acc_list[AccPart::FloatingItem2 as usize - enum_offset] =
                Self::compose_to_byte_array(
                    lv,
                    Self::roll_item_index(AccPartFileName::floatingItem, lv),
                )
        };

        // Roll ground item, mono or dual elements has different chance
        let p_group_gi = if element1 == element2 {
            ProbGroup::DUAL_SPC_GI_SAME
        } else {
            ProbGroup::DUAL_SPC_GI_DIFF
        };
        if roll_possess(p_group_gi) {
            let lv = Self::roll_lv(1, rarity_lv_cap);
            result_acc_list[AccPart::GroundItem2 as usize - enum_offset] =
                Self::compose_to_byte_array(
                    lv,
                    Self::roll_item_index(AccPartFileName::groundItem, lv),
                )
        };

        // Roll background effect, if mono_sp has acquired a background effect, this field should be ignored
        if roll_possess(ProbGroup::DUAL_SPC_BE) {
            let lv = Self::roll_lv(1, rarity_lv_cap);
            result_acc_list[AccPart::BackgroundEffect2 as usize - enum_offset] =
                Self::compose_to_byte_array(
                    lv,
                    Self::roll_item_index(AccPartFileName::backgroundEffect, lv),
                )
        };

        // [GroundEffect, FloatingItem2, GroundItem2, BackgroundEffect2]
        result_acc_list
    }

    /// Consecutivly pickup non-repeated multiple accessories. Not acquired accessory part will remain 0.
    ///
    /// And each item's rarity lv are different by acquiring order.
    fn pick_accessories(
        pick_num: usize,
        rarity_lv_cap: usize,
        remain_pool: &mut Vec<usize>,
    ) -> Vec<u32> {
        let enum_offset = remain_pool[0];
        let pool_len = remain_pool.len();
        let pick = cmp::min(pick_num, pool_len);
        let mut result_acc_byte_array = vec![0; pool_len];
        let mut is_first_pickup = true;

        while remain_pool.len() > pool_len - pick {
            let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
            let chosen_part = remain_pool[rand_holder.sample(..remain_pool.len() as u32) as usize];
            drop(rand_holder);

            remain_pool.retain(|acc| *acc != chosen_part);
            let lv = if is_first_pickup {
                // First picked accessory part guaranteed to be top rarity lv
                rarity_lv_cap
            } else {
                // Remaining accessory part are evenly pickup from all avaliable range from lowest rairity lv
                Self::roll_lv(1, rarity_lv_cap)
            };

            let item_index = Self::roll_item_index(AccPartFileName::from(chosen_part), lv);
            let byte_array = Self::compose_to_byte_array(lv, item_index);
            result_acc_byte_array[chosen_part - enum_offset] = byte_array;
            is_first_pickup = false;
        }

        result_acc_byte_array
    }

    /// Evaluating the top rarity lv can be acquired by character's attribute value
    fn get_rarity_lv_cap(val: u32, base_val: u32, val_interval: u32) -> usize {
        let rarity_value_interval = cmp::max(val_interval / MAX_RARITY_LV as u32, 1);
        cmp::min(
            ((val - base_val) / rarity_value_interval) + 1,
            MAX_RARITY_LV as u32,
        ) as usize
    }

    /// Evaluating the top rarity lv can be acquired by character's attribute value
    fn get_dual_spc_rarity_lv_cap(val: i32, config: &GameplayConfigManager) -> usize {
        let attr_config = config.get_char_attr_config();

        let rarity_value_interval = attr_config.dual_sp_gem_range / MAX_RARITY_LV as u32;
        // Dual special tile boost value has an exclusion interval, should deal the exclusion offset
        let offset_val = if val
            >= attr_config.dual_sp_gem_gap_start + attr_config.dual_sp_gem_gap_range as i32
        {
            val - attr_config.dual_sp_gem_gap_range as i32
        } else {
            val
        };

        let rarity_lv_cap = cmp::min(
            (offset_val - attr_config.dual_sp_gem_min) as u32 / rarity_value_interval + 1,
            MAX_RARITY_LV as u32,
        ) as usize;

        rarity_lv_cap
    }

    /// Byte array using 4 bytes => (Unused),(Unused),(item level),(item_index)
    fn compose_to_byte_array(lv: usize, item_index: u32) -> u32 {
        ((lv as u32) << 8) + item_index
    }

    fn roll_lv(low: usize, high: usize) -> usize {
        if high <= low {
            return std::cmp::max(low, high);
        }
        let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
        rand_holder.sample(low as u32..=high as u32) as usize
    }

    fn roll_eyes(high: usize) -> usize {
        let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
        // Special rarity of eyes is start from lv 3, so rarity lv 2 is using to indicate the primitive race eyes pool.
        // To ensure there is an uniform probability form primitive pool and high rarity pool.
        let mut eye_lv = rand_holder.sample(2..=high as u32);

        // If lv is roll to 2, roll the eyes again from 4 races
        if eye_lv == 2 {
            eye_lv = rand_holder
                .sample(PrimitiveEyes::PecoraEyes as u32..=PrimitiveEyes::CanidaeEyes as u32);
        }

        // Possible result: [11, 12, 13, 14, 3, 4, 5]
        eye_lv as usize
    }

    fn roll_item_index(part_name: AccPartFileName, lv: usize) -> u32 {
        if part_name == AccPartFileName::eye && (lv < 3 || lv > 5) {
            // Eyes is special case, only 1 primitive eyes
            return 1;
        }
        let max_item_index = ART_ASSET_AMOUNT.accessory[part_name as usize][lv];
        let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
        rand_holder.sample(..max_item_index) + 1
    }

    pub fn _debug_evenly_roll_accessory() -> Self {
        let mut accessory_list = vec![];
        let eye_lv_list = vec![11, 12, 13, 14, 3, 4, 5];

        for acc in AccPart::iter() {
            let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
            let lv = rand_holder.sample(1..=MAX_RARITY_LV as u32) as usize;
            let eye_lv = eye_lv_list[rand_holder.sample(..eye_lv_list.len() as u32) as usize];
            drop(rand_holder);

            match acc {
                AccPart::GroundEffect => accessory_list.push(Self::compose_to_byte_array(
                    lv,
                    Self::roll_item_index(AccPartFileName::groundEffect, lv),
                )),
                AccPart::FloatingItem1 | AccPart::FloatingItem2 => {
                    accessory_list.push(Self::compose_to_byte_array(
                        lv,
                        Self::roll_item_index(AccPartFileName::floatingItem, lv),
                    ))
                }
                AccPart::GroundItem1 | AccPart::GroundItem2 => {
                    accessory_list.push(Self::compose_to_byte_array(
                        lv,
                        Self::roll_item_index(AccPartFileName::groundItem, lv),
                    ))
                }
                AccPart::BackgroundEffect1 | AccPart::BackgroundEffect2 => {
                    accessory_list.push(Self::compose_to_byte_array(
                        lv,
                        Self::roll_item_index(AccPartFileName::backgroundEffect, lv),
                    ))
                }
                AccPart::Eyes => accessory_list.push(Self::compose_to_byte_array(
                    eye_lv,
                    Self::roll_item_index(AccPartFileName::eye, eye_lv),
                )),
                _ => accessory_list.push(Self::compose_to_byte_array(
                    lv,
                    Self::roll_item_index(AccPartFileName::from(acc as usize), lv),
                )),
            }
        }
        Self { accessory_list }
    }
}
