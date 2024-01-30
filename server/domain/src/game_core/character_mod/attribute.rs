use atb_types::prelude::uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::cmp;
use std::sync::RwLockWriteGuard;
use strum::EnumCount;

use crate::game_core::character_mod::char_const::*;
use crate::game_core::config::{
    ClearPattern, Element, GameplayConfigManager, TieredType, BOARD_HEIGHT, BOARD_WIDTH,
};
use crate::game_core::probability_mod::*;
use crate::game_core::skill::{ActivatingBuff, CharacterSkill, PassiveName, SkillInfo, SkillParam};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Attribute {
    id: Uuid,
    max_hp: u32,
    current_hp: u32,
    atk: u32,
    def: u32,
    element: Element,
    special_tile: SpecialTile,
    skill: CharacterSkill,
    passive: PassiveName,
    buff_states: Vec<ActivatingBuff>,
    assist_nerf_modifier: u32,
}

impl Attribute {
    pub fn roll_attribute(tier_lv: usize, config: &GameplayConfigManager) -> Self {
        let max_hp = Self::roll_max_hp(tier_lv, config);
        Attribute {
            id: Uuid::new_v4(),
            max_hp,
            current_hp: max_hp,
            atk: Self::roll_atk(tier_lv, config),
            def: Self::roll_def(tier_lv, config),
            element: Self::roll_element(),
            special_tile: Self::roll_special_tile(tier_lv, config),
            skill: Self::roll_skill(),
            passive: Self::roll_passive(tier_lv),
            buff_states: vec![],
            assist_nerf_modifier: config.get_assist_modifier_rate(),
        }

        /*
        let rand_holder = RANDOM_NUM_HOLDER.read().expect(LOCK_POISONED);
        log::debug!(
            "   ### Attribute used_bit:{}, rand_consumed: {}",
            rand_holder.bit_consumed,
            rand_holder.rand_consumed
        );
        */
    }

    pub fn scale_char_attributes(&mut self, hp_scale: f64, atk_scale: f64, def_scale: f64) {
        self.max_hp = (self.max_hp as f64 * hp_scale).round() as u32;
        self.current_hp = (self.current_hp as f64 * hp_scale).round() as u32;
        self.atk = (self.atk as f64 * atk_scale).round() as u32;
        self.def = (self.def as f64 * def_scale).round() as u32;
    }

    pub fn _debug_specify_roll_attribute(
        tier_lv: usize,
        assigned_element: &Option<Element>,
        assigned_skill: &Option<SkillInfo>,
        assigned_skill_param_elem: &Option<Element>,
        assigned_skill_param_clear_pattern: &Option<ClearPattern>,
        config: &GameplayConfigManager,
    ) -> Self {
        let max_hp = Self::roll_max_hp(tier_lv, config);

        let skill = assigned_skill.map_or_else(
            || Self::roll_skill(),
            |skill_name| {
                Self::_debug_assigned_skill(
                    skill_name,
                    assigned_skill_param_elem,
                    assigned_skill_param_clear_pattern,
                )
            },
        );

        Attribute {
            id: Uuid::new_v4(),
            max_hp,
            current_hp: max_hp,
            atk: Self::roll_atk(tier_lv, config),
            def: Self::roll_def(tier_lv, config),
            element: assigned_element.unwrap_or_else(|| Self::roll_element()),
            special_tile: Self::roll_special_tile(tier_lv, config),
            skill,
            passive: Self::roll_passive(tier_lv),
            buff_states: vec![],
            assist_nerf_modifier: config.get_assist_modifier_rate(),
        }
    }

    pub fn get_char_rarity(attribute: &Attribute, config: &GameplayConfigManager) -> u8 {
        let mut rarity_score = vec![];
        let attr_config = config.get_char_attr_config();

        rarity_score.push(Self::get_rarity_score(
            &attribute.max_hp,
            attr_config.hp_min,
            attr_config.hp_max - attr_config.hp_min,
        ));
        rarity_score.push(Self::get_rarity_score(
            &attribute.atk,
            attr_config.atk_min,
            attr_config.atk_max - attr_config.atk_min,
        ));
        rarity_score.push(Self::get_rarity_score(
            &attribute.def,
            attr_config.def_min,
            attr_config.def_max - attr_config.def_min,
        ));
        rarity_score.push(Self::get_spc_rarity_score(&attribute.special_tile, config));
        rarity_score.push(match attribute.passive {
            PassiveName::None => 0,
            _ => 5,
        });

        cmp::max(1, rarity_score.iter().sum::<u8>() / RARITY_SLOT)
    }

    pub fn get_id(&self) -> &Uuid {
        &self.id
    }

    pub fn get_max_hp(&self) -> u32 {
        self.max_hp
    }

    pub fn set_max_hp(&mut self, param: u32) {
        self.max_hp = param;
        self.current_hp = param;
    }

    pub fn get_atk(&self) -> u32 {
        self.atk
    }

    pub fn get_def(&self) -> u32 {
        self.def
    }

    pub fn get_element(&self) -> Element {
        self.element
    }

    pub fn set_element(&mut self, param: Element) {
        self.element = param;
    }

    pub fn get_assist_nerf_modifier(&self) -> u32 {
        self.assist_nerf_modifier
    }

    pub fn get_special_tile(&self) -> &SpecialTile {
        &self.special_tile
    }

    pub fn set_skill_meta(&mut self, skill_info: SkillInfo) {
        let rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
        let param = Self::roll_skill_param(rand_holder, skill_info);

        self.skill = CharacterSkill::new(skill_info, 0, param);
    }

    pub fn get_skill_meta(&self) -> &CharacterSkill {
        &self.skill
    }

    pub fn get_passive(&self) -> &PassiveName {
        &self.passive
    }

    fn get_rarity_score(val: &u32, base: u32, interval: u32) -> u8 {
        let slot_interval = cmp::max(interval / RARITY_SLOT as u32, 1);
        ((val - base) / slot_interval) as u8
    }

    fn get_spc_rarity_score(special_tile: &SpecialTile, config: &GameplayConfigManager) -> u8 {
        if special_tile.element1 == Element::Unknown {
            return RARITY_NO_SPC_SCORE;
        }

        let attr_config = config.get_char_attr_config();
        let mono_spc_score = Self::get_rarity_score(
            &special_tile.elem1_boost_val,
            attr_config.mono_sp_gem_min,
            attr_config.mono_sp_gem_max - attr_config.mono_sp_gem_min,
        );

        if special_tile.element2 == Element::Unknown {
            return mono_spc_score;
        }
        mono_spc_score + RARITY_DUAL_SPC_SCORE
    }

    fn roll_max_hp(tier_lv: usize, config: &GameplayConfigManager) -> u32 {
        let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
        let hp = config.get_tier_range(TieredType::HP);
        rand_holder.sample(hp.tier_min[tier_lv]..=hp.tier_max[tier_lv])
    }

    fn roll_atk(tier_lv: usize, config: &GameplayConfigManager) -> u32 {
        let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
        let atk = config.get_tier_range(TieredType::ATK);
        rand_holder.sample(atk.tier_min[tier_lv]..=atk.tier_max[tier_lv])
    }

    fn roll_def(tier_lv: usize, config: &GameplayConfigManager) -> u32 {
        let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
        let def = config.get_tier_range(TieredType::DEF);
        rand_holder.sample(def.tier_min[tier_lv]..=def.tier_max[tier_lv])
    }

    fn roll_element() -> Element {
        let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
        // Subtract 1 COUNT is Element::Unknown
        Element::from(rand_holder.sample(..(Element::COUNT - 1) as u32))
    }

    fn roll_skill() -> CharacterSkill {
        let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
        let info = SkillInfo::from(rand_holder.sample(SkillInfo::available_skill_range()));
        let param = Self::roll_skill_param(rand_holder, info);

        CharacterSkill::new(info, 0, param)
    }

    fn roll_skill_param(
        mut rand_holder: RwLockWriteGuard<RandomNumHolder>,
        skill_info: SkillInfo,
    ) -> SkillParam {
        match skill_info {
            info @ SkillInfo::ElementalExplosion => {
                let skill_param_element =
                    Element::from(rand_holder.sample(..(Element::COUNT - 1) as u32));
                SkillParam::new(info, None, Some(skill_param_element), None)
            }
            info @ SkillInfo::LineEliminate => {
                let skill_param_clear_pattern =
                    ClearPattern::from(rand_holder.sample(1..=2) as u32);

                let max_value = match skill_param_clear_pattern {
                    ClearPattern::Horizontal => BOARD_HEIGHT,
                    ClearPattern::Vertical => BOARD_WIDTH,
                    _ => unreachable!(),
                };

                SkillParam::new(
                    info,
                    Some(rand_holder.sample(0..max_value) as u32),
                    None,
                    Some(skill_param_clear_pattern),
                )
            }
            info => SkillParam::new(info, None, None, None),
        }
    }

    fn roll_passive(tier_lv: usize) -> PassiveName {
        let mut passive = PassiveName::default();
        if roll_possess(ProbGroup::PASSIVE(tier_lv)) {
            let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
            passive = PassiveName::from(rand_holder.sample(..(PassiveName::COUNT) as u32));
        }
        passive
    }

    fn roll_special_tile(tier_lv: usize, config: &GameplayConfigManager) -> SpecialTile {
        let mut special_tile = SpecialTile::new();

        // Roll first special tile
        if roll_possess(ProbGroup::MONO_SPC_TILE(tier_lv)) {
            // Roll boost element and value
            let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
            let element1 = rand_holder.sample(..(Element::COUNT - 1) as u32);

            let mono_sp_gem = config.get_tier_range(TieredType::MONO_SP_GEM);
            let elem1_boost_val =
                rand_holder.sample(mono_sp_gem.tier_min[tier_lv]..=mono_sp_gem.tier_max[tier_lv]);
            special_tile.set_element1(Element::from(element1), elem1_boost_val);
            drop(rand_holder);

            // Roll second special tile
            if roll_possess(ProbGroup::DUAL_SPC_TILE(tier_lv)) {
                // Roll boost element and value
                let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);
                let element2 = rand_holder.sample(..(Element::COUNT - 1) as u32);

                let attr_config = config.get_char_attr_config();

                // TODO: should adjust the rule in future design
                let mut elem2_boost_val = attr_config.dual_sp_gem_min
                    + rand_holder.sample(..=attr_config.dual_sp_gem_range) as i32;

                // Offset the exclusion interval (no value between range -30~30)
                if elem2_boost_val > attr_config.dual_sp_gem_gap_start {
                    elem2_boost_val += attr_config.dual_sp_gem_range as i32;
                }
                special_tile.set_element2(Element::from(element2), elem2_boost_val);
            }
        }
        special_tile
    }

    pub fn _debug_assigned_skill(
        info: SkillInfo,
        assigned_skill_param_elem: &Option<Element>,
        assigned_skill_param_clear_pattern: &Option<ClearPattern>,
    ) -> CharacterSkill {
        let param = Self::_debug_roll_skill_param(
            info,
            &assigned_skill_param_elem,
            &assigned_skill_param_clear_pattern,
        );

        CharacterSkill::new(info, 0, param)
    }

    fn _debug_roll_skill_param(
        skill_info: SkillInfo,
        assigned_element: &Option<Element>,
        assigned_clear_pattern: &Option<ClearPattern>,
    ) -> SkillParam {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();

        match skill_info {
            info @ SkillInfo::ElementalExplosion => {
                let element = assigned_element.unwrap_or_else(|| {
                    Element::from(rng.gen_range(0..(Element::COUNT - 1)) as u32)
                });

                SkillParam::new(info, None, Some(element), None)
            }
            info @ SkillInfo::LineEliminate => {
                let clear_pattern = assigned_clear_pattern
                    .unwrap_or_else(|| ClearPattern::from(rng.gen_range(1..=2) as u32));

                let value = match clear_pattern {
                    ClearPattern::Vertical => rng.gen_range(0..BOARD_WIDTH),
                    ClearPattern::Horizontal => rng.gen_range(0..BOARD_HEIGHT),
                    _ => unreachable!(),
                };

                SkillParam::new(info, Some(value), None, Some(clear_pattern))
            }
            info => SkillParam::new(info, None, None, None),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SpecialTile {
    pub element1: Element,
    pub element2: Element,
    pub elem1_boost_val: u32,
    pub elem2_boost_val: i32,
}

impl SpecialTile {
    pub fn new() -> Self {
        Self {
            element1: Element::Unknown,
            element2: Element::Unknown,
            elem1_boost_val: Default::default(),
            elem2_boost_val: Default::default(),
        }
    }

    pub fn set_element1(&mut self, element: Element, boost_val: u32) {
        self.element1 = element;
        self.elem1_boost_val = boost_val;
    }

    pub fn set_element2(&mut self, element: Element, boost_val: i32) {
        self.element2 = element;
        self.elem2_boost_val = boost_val;
    }
}
