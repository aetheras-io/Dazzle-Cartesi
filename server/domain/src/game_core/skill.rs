use super::config::{ClearPattern, Element};
use super::lazy_static;
use atb::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::{RangeBounds, RangeInclusive};
use strum_macros::{EnumCount, EnumIter, EnumString};

use crate::game_core::GameError;

lazy_static::lazy_static! {
    pub static ref SKILL_PARAM_CONFIG: SkillParamConfig = serde_json::from_slice(include_bytes!("./config/skill_param_table.json")).expect("can't not parse SKILL_PARAM_CONFIG config");
}

#[derive(Debug, Clone, Deserialize, Serialize, EnumCount, PartialEq)]
pub enum PassiveName {
    Passive0,
    Passive1,
    None,
}

impl Default for PassiveName {
    fn default() -> Self {
        Self::None
    }
}

impl From<u32> for PassiveName {
    fn from(v: u32) -> Self {
        match v {
            0 => Self::Passive0,
            1 => Self::Passive1,
            2 => Self::None,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillParamConfig {
    pub skill_param_table: HashMap<SkillInfo, SkillParam>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct SkillParam {
    // Fixed value from config file
    active_turns: u8,
    consumable_amount: u32,
    energy_per_cast: u32,
    max_stack: u32,
    charge_rate: u32,
    enable_clear_bead_damage: Option<bool>, // Decide the damage should be calculated from the beads that are eliminated by the skill.
    enable_falling_clear_damage: Option<bool>, // Decide the damage should be calculated from the beads that are falling cleared in skill stage.

    // Could from to config or assigned at runtime from character data, depending on what the skill is.
    value: Option<u32>,

    // Assigned at runtime from character data, not exist in config
    max_skill_charge: Option<u32>,
    element: Option<Element>,
    clear_pattern: Option<ClearPattern>,
}

impl SkillParam {
    pub fn new(
        info: SkillInfo,
        value: Option<u32>,
        element: Option<Element>,
        clear_pattern: Option<ClearPattern>,
    ) -> Self {
        Self {
            energy_per_cast: info.get_config_energy_per_cast(),
            max_stack: info.get_config_max_stack(),
            max_skill_charge: Some(info.get_config_energy_per_cast() * info.get_config_max_stack()),
            charge_rate: info.get_config_charge_rate(),
            active_turns: info.get_config_active_turns(),
            consumable_amount: info.get_config_consumable_amount(),
            enable_clear_bead_damage: Default::default(),
            enable_falling_clear_damage: Default::default(),
            value: value.or_else(|| Some(info.get_config_value())),
            element,
            clear_pattern,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CharacterSkill {
    info: SkillInfo,
    cool_down: u32,
    param: SkillParam,
}

impl CharacterSkill {
    pub fn new(info: SkillInfo, cool_down: u32, param: SkillParam) -> Self {
        Self {
            info,
            cool_down,
            param,
        }
    }

    pub fn is_skill_ready(&self) -> bool {
        self.cool_down >= self.get_energy_per_cast()
    }

    pub fn get_skill_info(&self) -> SkillInfo {
        self.info
    }

    pub fn get_current_cool_down(&self) -> u32 {
        self.cool_down
    }

    pub fn set_skill_cool_down(&mut self, charged_val: u32) {
        self.cool_down = charged_val
    }

    pub fn set_testing_init_skill_cool_down(&mut self, filled_rate: u32) {
        self.cool_down =
            (self.get_max_skill_charge() as f64 * filled_rate as f64 / 100.0).round() as u32;
    }

    pub fn get_energy_per_cast(&self) -> u32 {
        self.param.energy_per_cast
    }

    pub fn get_max_skill_charge(&self) -> u32 {
        self.param.max_skill_charge.unwrap_or_default()
    }

    pub fn get_param_value(&self) -> u32 {
        self.param
            .value
            .unwrap_or_else(|| self.info.get_config_value())
    }

    pub fn get_param_element(&self) -> Option<Element> {
        self.param
            .element
            .or_else(|| self.info.get_config_element())
    }

    pub fn get_param_clear_pattern(&self) -> Option<ClearPattern> {
        self.param
            .clear_pattern
            .or_else(|| self.info.get_config_clear_pattern())
    }
}

#[derive(
    Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash, EnumIter, EnumCount, EnumString,
)]
pub enum SkillInfo {
    #[strum(serialize = "replacetestboard")]
    ReplaceTestBoard, // Feature for testing
    #[strum(serialize = "damage")]
    Damage,
    #[strum(serialize = "recovery")]
    Recovery,
    #[strum(serialize = "defenseamplify")]
    DefenseAmplify,
    #[strum(serialize = "turntiles")]
    TurnTiles,
    #[strum(serialize = "attackamplify")]
    AttackAmplify,
    #[strum(serialize = "shieldnullify")]
    ShieldNullify,
    #[strum(serialize = "shieldabsorb")]
    ShieldAbsorb,
    #[strum(serialize = "elementalexplosion")]
    ElementalExplosion,
    #[strum(serialize = "lineeliminate")]
    LineEliminate,
    #[strum(serialize = "npcattack")]
    NpcAttack, // "NPC Attack" is not a real available skill. It is a convenient practice for PvE enemy normal attack.
    None, // "None" needs always be the last one
}

impl SkillInfo {
    pub fn is_borad_skill(&self) -> bool {
        match self {
            Self::TurnTiles | Self::ElementalExplosion | Self::LineEliminate => true,
            _ => false,
        }
    }

    pub fn get_config_energy_per_cast(&self) -> u32 {
        self.get_config(|param| param.energy_per_cast)
    }

    pub fn get_config_max_stack(&self) -> u32 {
        self.get_config(|param| param.max_stack)
    }

    pub fn get_config_charge_rate(&self) -> u32 {
        self.get_config(|param| param.charge_rate)
    }

    pub fn get_config_active_turns(&self) -> u8 {
        self.get_config(|param| param.active_turns)
    }

    pub fn get_config_consumable_amount(&self) -> u32 {
        self.get_config(|param| param.consumable_amount)
    }

    pub fn get_config_value(&self) -> u32 {
        self.get_config(|param| param.value).unwrap_or_default()
    }

    pub fn get_config_element(&self) -> Option<Element> {
        self.get_config(|param| param.element)
    }

    pub fn get_config_clear_pattern(&self) -> Option<ClearPattern> {
        self.get_config(|param| param.clear_pattern)
    }

    pub fn is_clear_bead_produce_damage(&self) -> bool {
        self.get_config(|param| param.enable_clear_bead_damage)
            .unwrap_or(false)
    }

    pub fn is_falling_clear_produce_damage(&self) -> bool {
        self.get_config(|param| param.enable_falling_clear_damage)
            .unwrap_or(false)
    }

    pub fn available_skill_range() -> impl RangeBounds<u32> {
        SkillInfo::Damage as u32..=SkillInfo::LineEliminate as u32
    }

    pub fn random_enemy_command_range() -> RangeInclusive<u32> {
        SkillInfo::Damage as u32..=SkillInfo::NpcAttack as u32
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => SkillInfo::ReplaceTestBoard,
            1 => SkillInfo::Damage,
            2 => SkillInfo::Recovery,
            3 => SkillInfo::DefenseAmplify,
            4 => SkillInfo::TurnTiles,
            5 => SkillInfo::AttackAmplify,
            6 => SkillInfo::ShieldNullify,
            7 => SkillInfo::ShieldAbsorb,
            8 => SkillInfo::ElementalExplosion,
            9 => SkillInfo::LineEliminate,
            10 => SkillInfo::NpcAttack,
            _ => SkillInfo::None,
        }
    }

    fn get_config<F, T>(&self, get_param: F) -> T
    where
        F: FnOnce(&SkillParam) -> T,
        T: Default,
    {
        SKILL_PARAM_CONFIG.skill_param_table.get(self).map_or_else(
            || {
                log::error!("{}: Field {:?}", GameError::SkillParamError, self);
                T::default()
            },
            get_param,
        )
    }
}

impl From<u32> for SkillInfo {
    fn from(v: u32) -> Self {
        match v {
            0 => Self::ReplaceTestBoard,
            1 => Self::Damage,
            2 => Self::Recovery,
            3 => Self::DefenseAmplify,
            4 => Self::TurnTiles,
            5 => Self::AttackAmplify,
            6 => Self::ShieldNullify,
            7 => Self::ShieldAbsorb,
            8 => Self::ElementalExplosion,
            9 => Self::LineEliminate,
            10 => Self::None,
            _ => unreachable!(),
        }
    }
}

impl From<&BuffInfo> for SkillInfo {
    fn from(b: &BuffInfo) -> Self {
        match b {
            BuffInfo::None => Self::None,
            BuffInfo::DefenseAmplify => Self::DefenseAmplify,
            BuffInfo::AttackAmplify => Self::AttackAmplify,
            BuffInfo::ShieldNullify => Self::ShieldNullify,
            BuffInfo::ShieldAbsorb => Self::ShieldAbsorb,
        }
    }
}

// Buff state implement in bitmask for future extension (0,1,2,4,8,16 ...)
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum BuffInfo {
    None,
    DefenseAmplify,
    AttackAmplify,
    ShieldNullify,
    ShieldAbsorb,
}

impl BuffInfo {
    pub fn get_value(&self) -> u32 {
        SkillInfo::from(self).get_config_value()
    }

    pub fn get_active_turns(&self) -> u8 {
        let turns = SkillInfo::from(self).get_config_active_turns();
        if turns == 0 {
            log::debug!("{}", GameError::SkillParamError);
            return 0;
        }

        // Buff type skill should skip opponent's turns, so active turns need mutiple 2
        if self.is_denfense_type() {
            (turns.saturating_mul(2) as u8).saturating_sub(1)
        } else {
            ((turns - 1).saturating_mul(2) as u8).saturating_sub(1)
        }
    }

    pub fn get_consumable_amount(&self) -> u32 {
        SkillInfo::from(self).get_config_consumable_amount()
    }

    pub fn bitmask(&self) -> u32 {
        match *self {
            Self::None => 0,
            Self::DefenseAmplify => 1,
            Self::AttackAmplify => 1 << 1,
            Self::ShieldNullify => 1 << 2,
            Self::ShieldAbsorb => 1 << 3,
        }
    }

    pub fn is_consumable_type(&self) -> bool {
        match self {
            Self::ShieldNullify | Self::ShieldAbsorb => true,
            _ => false,
        }
    }

    pub fn is_shield_type(&self) -> bool {
        match self {
            Self::ShieldNullify | Self::ShieldAbsorb => true,
            _ => false,
        }
    }

    // Workaroud solution, more details in #411
    pub fn is_denfense_type(&self) -> bool {
        match self {
            Self::DefenseAmplify => true,
            _ => self.is_shield_type(),
        }
    }
}

impl From<SkillInfo> for BuffInfo {
    fn from(info: SkillInfo) -> Self {
        match info {
            SkillInfo::DefenseAmplify => Self::DefenseAmplify,
            SkillInfo::AttackAmplify => Self::AttackAmplify,
            SkillInfo::ShieldNullify => Self::ShieldNullify,
            SkillInfo::ShieldAbsorb => Self::ShieldAbsorb,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActivatingBuff {
    pub buff: BuffInfo,
    pub effect_value: u32,
    pub consumable_amount: u8,
    pub end_turn: u8,
}
