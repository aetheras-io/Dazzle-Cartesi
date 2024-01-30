use atb::prelude::*;
use atb_types::Uuid;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::cmp;
use std::collections::HashMap;
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::EnumString;

use super::skill::PassiveName;
use crate::game_core::character_mod::accessory_module::{AccPart, AccessoryModule};
use crate::game_core::character_mod::attribute::{Attribute, SpecialTile};
use crate::game_core::character_mod::base_body_module::BaseBodyModule;
use crate::game_core::config::{
    ClearPattern, Element, GameplayConfigManager, BOSS_ENEMY_STRING, ELITE_ENEMY_STRING,
    NORMAL_ENEMY_STRING, RATE_UNIT,
};
use crate::game_core::skill::{ActivatingBuff, BuffInfo, CharacterSkill, SkillInfo};
use crate::game_core::GameError;

//#Note: Use in Unity client & Game logic only for Room data without any character visual data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CharacterLogicData {
    pub id: Uuid,
    #[serde(skip)]
    pub max_hp: u32,
    pub current_hp: u32,
    #[serde(skip)]
    pub atk: u32,
    #[serde(skip)]
    pub def: u32,
    pub element: Element,
    #[serde(skip)]
    pub special_tile: SpecialTile,
    pub skill: CharacterSkill,
    #[serde(skip)]
    pub passive: PassiveName,
    pub buff_states: Vec<ActivatingBuff>,
    #[serde(skip)]
    pub assist_nerf_modifier: u32,
}

impl CharacterLogicData {
    pub fn update_hp(&mut self, hp: i32) {
        self.update_current_hp(cmp::min(self.max_hp, cmp::max(0, hp) as u32));

        if self.current_hp == 0 {
            self.reset_cool_down();
        }
    }

    pub fn update_current_hp(&mut self, hp: u32) {
        self.current_hp = cmp::min(hp, self.max_hp);
    }

    pub fn recovery_hp(&mut self, recovery_val: u32) -> bool {
        if !self.is_alive() {
            return false;
        }

        self.update_current_hp(self.current_hp + recovery_val);

        log::debug!("   # Recovery[{}] value: {}", self.id, recovery_val);
        true
    }

    pub fn add_buff_states(&mut self, buff: BuffInfo, current_turn: u8) {
        let char_id = &self.id;
        let consumable_amount = buff.get_consumable_amount() as u8;
        let active_turns = buff.get_active_turns();
        match self.buff_states.iter_mut().find(|b| b.buff == buff) {
            Some(b) => {
                // Already activated, only extend expired time or consumable amounts
                if buff.is_consumable_type() {
                    b.consumable_amount = b.consumable_amount.saturating_add(consumable_amount);
                    log::debug!(
                        "   # Buff[{:?}] at char: [{}], consumable amount: {}",
                        buff,
                        char_id,
                        b.consumable_amount,
                    );
                } else {
                    b.end_turn = b.end_turn.saturating_add(active_turns);
                    log::debug!(
                        "   # Buff[{:?}] at char: [{}], expire after: {}",
                        buff,
                        char_id,
                        b.end_turn,
                    );
                }
            }
            None => {
                // Add new buff effect
                let end_turn = current_turn.saturating_add(active_turns);
                log::debug!(
                    "   # Buff[{:?}] at char: [{}], expire after: {}",
                    buff,
                    char_id,
                    end_turn,
                );

                let effect_value = match buff {
                    BuffInfo::None => return,
                    BuffInfo::DefenseAmplify => {
                        let damage_reduction = self.max_hp * buff.get_value() / RATE_UNIT;
                        log::debug!("      # reduce damage: {}", damage_reduction);
                        damage_reduction
                    }
                    BuffInfo::AttackAmplify => {
                        let attack_amplify_rate = buff.get_value();
                        log::debug!("      # atk amplify rate: {}", attack_amplify_rate);
                        attack_amplify_rate
                    }
                    BuffInfo::ShieldNullify => {
                        log::debug!("      # nullify damage, amount: {}", consumable_amount);
                        Default::default()
                    }
                    BuffInfo::ShieldAbsorb => {
                        let absorb_rate = buff.get_value();
                        log::debug!(
                            "      # absorb damage rate: {}, amount: {}",
                            absorb_rate,
                            consumable_amount
                        );
                        absorb_rate
                    }
                };

                let new_buff = ActivatingBuff {
                    buff,
                    effect_value,
                    consumable_amount,
                    end_turn,
                };

                // ### SPEC?: Exclusive shield, only one shield type buff exist at same time. Overwrite with the latest triggered one.
                let existing_shield_buff = self
                    .buff_states
                    .iter_mut()
                    .find(|b| b.buff.is_shield_type());

                if new_buff.buff.is_shield_type() && existing_shield_buff.is_some() {
                    log::debug!("   # Overwrite buff: {:?}", new_buff.buff);
                    *existing_shield_buff.unwrap() = new_buff;
                } else {
                    log::debug!("   # Push buff: {:?}", new_buff.buff);
                    self.buff_states.push(new_buff);
                }
            }
        }
    }

    pub fn update_cool_down(&mut self, removed_beads: &[u32], config: &GameplayConfigManager) {
        if !self.is_alive() {
            return;
        }
        self.skill.set_skill_cool_down(cmp::min(
            self.get_max_skill_charge(),
            self.get_current_cool_down()
                + self.eval_skill_charge_by_clear(removed_beads, config)
                + config.get_auto_charge_each_turn(),
        ))
    }

    pub fn add_extra_cool_down(&mut self, extra_val: u32) {
        if !self.is_alive() {
            return;
        }
        self.skill.set_skill_cool_down(cmp::min(
            self.get_max_skill_charge(),
            self.get_current_cool_down() + extra_val,
        ))
    }

    pub fn get_max_skill_charge(&self) -> u32 {
        self.skill.get_max_skill_charge()
    }

    pub fn reset_cool_down(&mut self) {
        self.skill.set_skill_cool_down(0)
    }

    pub fn consume_cool_down(&mut self) {
        let remain_charge_val = self
            .get_current_cool_down()
            .saturating_sub(self.skill.get_energy_per_cast());

        self.skill.set_skill_cool_down(remain_charge_val)
    }

    pub fn alive(&self) -> Result<(), GameError> {
        if !self.is_alive() {
            return Err(GameError::IllegalMove);
        }
        Ok(())
    }

    pub fn is_alive(&self) -> bool {
        self.current_hp != 0
    }

    pub fn get_skill_info(&self) -> SkillInfo {
        self.skill.get_skill_info()
    }

    pub fn get_current_cool_down(&self) -> u32 {
        self.skill.get_current_cool_down()
    }

    pub fn is_skill_ready(&self) -> bool {
        self.skill.is_skill_ready()
    }

    pub fn eval_skill_charge_by_clear(
        &self,
        removed_beads: &[u32],
        config: &GameplayConfigManager,
    ) -> u32 {
        let charge_info = config.get_charge_info();

        let clear_unit = charge_info.clear_unit;
        let basic_charge_rate = charge_info.diff_color_multiply;
        let boost_charge_rate_diff = charge_info.self_color_multiply - basic_charge_rate;

        // TODO: "get_skill_info().get_config_charge_rate()" Can be deprecate.
        //        Remove it completely while implement custom skill config.
        // let charge_rate = self.get_skill_info().get_config_charge_rate();

        let basic_charge =
            (removed_beads.iter().sum::<u32>() as f64 * clear_unit as f64 * basic_charge_rate)
                .round() as u32;

        let boost_charge = (removed_beads[self.element as usize] as f64
            * clear_unit as f64
            * boost_charge_rate_diff)
            .round() as u32;

        // log::warn!(" ### basic charge:{}", basic_charge);
        // log::warn!("    ### boost charge:{}\n", boost_charge);

        basic_charge + boost_charge
    }

    pub fn get_skill_param_value(&self) -> u32 {
        self.skill.get_param_value()
    }

    pub fn get_skill_target_elem(&self) -> Option<Element> {
        self.skill.get_param_element()
    }

    pub fn get_skill_clear_pattern(&self) -> Option<ClearPattern> {
        self.skill.get_param_clear_pattern()
    }

    pub fn get_amplify_buff_value(&self, buff_type: BuffInfo) -> u32 {
        self.buff_states
            .iter()
            .find(|b| b.buff == buff_type)
            .map(|amplify_buff| {
                log::debug!("        Extra buff {}", amplify_buff.effect_value);
                amplify_buff.effect_value
            })
            .unwrap_or(0)
    }

    // Sum up primitive atk and buffed atk value
    pub fn get_total_atk(&self) -> u32 {
        self.atk + self.get_amplify_buff_value(BuffInfo::AttackAmplify)
    }

    // Sum up primitive def and buffed def value
    pub fn get_total_def(&self) -> u32 {
        self.def + self.get_amplify_buff_value(BuffInfo::DefenseAmplify)
    }

    pub fn apply_shield_buff(&self, defender_received_damage: i32) -> (i32, bool) {
        if let Some(shield_type_buff) = self.buff_states.iter().find(|b| b.buff.is_shield_type()) {
            let result_damage = match shield_type_buff.buff {
                BuffInfo::ShieldNullify => {
                    log::debug!("      Damage Nullify triggered id:[{}]", self.id);
                    0
                }
                BuffInfo::ShieldAbsorb => {
                    let absorbed_damage = (defender_received_damage
                        * shield_type_buff.buff.get_value() as i32
                        / RATE_UNIT as i32)
                        * -1;

                    log::debug!(
                        "      Damage Absorb triggered id:[{}], absorb damage: {}",
                        self.id,
                        absorbed_damage
                    );

                    absorbed_damage
                }
                _ => unreachable!(),
            };

            return (result_damage, true);
        }

        (defender_received_damage, false)
    }

    pub fn consume_buff(&mut self) {
        if let Some(activating_buff) = self
            .buff_states
            .iter_mut()
            .find(|a| a.buff.is_consumable_type())
        {
            activating_buff.consumable_amount = activating_buff.consumable_amount.saturating_sub(1);
            if activating_buff.consumable_amount == 0 {
                log::debug!("   # remove: {:?}", activating_buff.buff)
            };
        }
    }

    pub fn remove_expired_buff_states(&mut self, current_turn: u8) {
        self.buff_states
            .retain(|buff| current_turn <= buff.end_turn && 0 < buff.consumable_amount);
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CharacterV2 {
    pub rarity: u8,
    pub attribute: Attribute,
    pub body_module: BaseBodyModule,
    pub accessory_module: AccessoryModule,
}

impl CharacterV2 {
    pub fn roll_new(tier_lv: usize, config: &GameplayConfigManager) -> Self {
        let attribute = Attribute::roll_attribute(tier_lv, config);
        let new_char = Self {
            accessory_module: AccessoryModule::roll_accessory(&attribute, config),
            body_module: BaseBodyModule::roll_base_body_module(),
            rarity: Attribute::get_char_rarity(&attribute, config),
            attribute,
        };

        //new_char._debug_log_print();
        new_char
    }

    pub fn create_tutorial_char(
        is_player: bool,
        element: Element,
        config: &GameplayConfigManager,
    ) -> Self {
        let mut attribute = Attribute::roll_attribute(1, config);

        // The accessory is highly coupled with the accessory module, need to clone attribute to initialize the accessory first
        let accessory_module = AccessoryModule::roll_accessory(&attribute.clone(), config);

        attribute.set_element(element);

        if is_player {
            attribute.set_skill_meta(SkillInfo::Damage);
        } else {
            attribute.set_max_hp(10);
        }

        let new_char = Self {
            // Note: The modules can be manually assigned in the future.
            accessory_module,
            body_module: BaseBodyModule::roll_base_body_module(),
            rarity: 1,
            attribute,
        };

        //new_char._debug_log_print();
        new_char
    }

    pub fn create_enemy_character(enemy_template: &EnemyTemplate, rift_lv: u32) -> Self {
        let mut new_char = CharacterV2::roll_new(0, &enemy_template.char_config);
        new_char.enemy_attribute_scaler(enemy_template, rift_lv);
        log::debug!("Genarate enemy character in rift lv: {}", rift_lv);
        log::debug!("New enemy character after scale:\n{:#?}", new_char);
        new_char
    }

    pub fn enemy_attribute_scaler(&mut self, template: &EnemyTemplate, rift_lv: u32) {
        let enemy_type = template.enemy_type;

        // Scaling factor: (hp, atk, def)
        let scaling_factor = match enemy_type {
            // NOTE: The coefficients are temporary a const.
            //       It will implement a config like tool to scale it flexible in the future.
            EnemyType::Boss => (5.0, 1.5, 1.5),
            EnemyType::Elite => (1.2, 1.0, 1.0),
            EnemyType::Normal => (0.8, 0.5, 0.5),
        };

        let hp_scale = scaling_factor.0 * (1.0 + template.lift_rate.hp * rift_lv as f64);
        let atk_scale = scaling_factor.1 * (1.0 + template.lift_rate.atk * rift_lv as f64);
        let def_scale = scaling_factor.2 * (1.0 + template.lift_rate.def * rift_lv as f64);

        log::debug!(
            "Scale: (hp: {:.3}, atk: {:.3}, def: {:.3})",
            hp_scale,
            atk_scale,
            def_scale
        );

        self.attribute
            .scale_char_attributes(hp_scale, atk_scale, def_scale)
    }

    pub fn reward_attribute_scaler(&mut self, rift_lv: u32) {
        let lift_rate_hp: f64 = 0.1;
        let lift_rate_atk: f64 = 0.1;
        let lift_rate_def: f64 = 0.1;

        let hp_scale = 1.0 + lift_rate_hp * rift_lv as f64;
        let atk_scale = 1.0 + lift_rate_atk * rift_lv as f64;
        let def_scale = 1.0 + lift_rate_def * rift_lv as f64;

        log::debug!(
            "Scale: (hp: {:.3}, atk: {:.3}, def: {:.3})",
            hp_scale,
            atk_scale,
            def_scale
        );

        self.attribute
            .scale_char_attributes(hp_scale, atk_scale, def_scale)
    }

    pub fn _debug_specify_roll_new(
        tier_lv: usize,
        req_element: &Option<String>,
        req_skill: &Option<String>,
        req_skill_param_elem: &Option<String>,
        req_skill_param_clear_pattern: &Option<String>,
        config: &GameplayConfigManager,
    ) -> Self {
        let (
            assigned_element,
            assigned_skill,
            assigned_skill_param_elem,
            assigned_skill_param_clear_pattern,
        ) = Self::_debug_deserialize_specify_field(
            req_element,
            req_skill,
            req_skill_param_elem,
            req_skill_param_clear_pattern,
        );

        let attribute = Attribute::_debug_specify_roll_attribute(
            tier_lv,
            &assigned_element,
            &assigned_skill,
            &assigned_skill_param_elem,
            &assigned_skill_param_clear_pattern,
            config,
        );

        let new_char = Self {
            accessory_module: AccessoryModule::roll_accessory(&attribute, config),
            body_module: BaseBodyModule::roll_base_body_module(), // Base body is already rolled evenly in currennt SPEC
            rarity: Attribute::get_char_rarity(&attribute, config),
            attribute,
        };

        //new_char._debug_log_print();
        new_char
    }

    fn _debug_deserialize_specify_field(
        req_element: &Option<String>,
        req_skill: &Option<String>,
        req_skill_param_elem: &Option<String>,
        req_skill_param_clear_pattern: &Option<String>,
    ) -> (
        Option<Element>,
        Option<SkillInfo>,
        Option<Element>,
        Option<ClearPattern>,
    ) {
        let assigned_element = req_element
            .as_ref()
            .filter(|s| !s.is_empty())
            .and_then(|s| Element::from_str(s).ok());

        let assigned_skill = req_skill.as_ref().and_then(|s| SkillInfo::from_str(s).ok());

        let assigned_skill_param_elem = req_skill_param_elem
            .as_ref()
            .and_then(|s| Element::from_str(s).ok());

        let assigned_skill_param_clear_pattern = req_skill_param_clear_pattern
            .as_ref()
            .and_then(|s| ClearPattern::from_str(s).ok());

        log::debug!(
            "   Assigned part - Element: {:?}, Skill: {:?}, Param_elem: {:?}, Param_clear_pattern: {:?}",
            assigned_element,
            assigned_skill,
            assigned_skill_param_elem,
            assigned_skill_param_clear_pattern
        );

        (
            assigned_element,
            assigned_skill,
            assigned_skill_param_elem,
            assigned_skill_param_clear_pattern,
        )
    }

    fn _debug_log_print(&self) {
        let mut _log = format!("\n --- Rarity: {} ---", self.rarity);
        _log += &self._debug_print_base_body();
        _log += &self._debug_print_accessory();
        log::debug!("{}", _log);
    }

    fn _debug_print_base_body(&self) -> String {
        let mut log = format!("\n\nBase Body -\n");
        log += &format!(
            "Race: {:?}, style: {}",
            self.body_module.style.race, self.body_module.style.body
        );
        log
    }

    fn _debug_print_accessory(&self) -> String {
        const ITEM_INDEX_MASK: u32 = 255;
        const LV_MASK: u32 = ITEM_INDEX_MASK << 8;
        let mut log = format!("\n\nAccessories -\n");
        for part in AccPart::iter() {
            if self.accessory_module.accessory_list[part as usize] != 0 {
                let lv = (LV_MASK & self.accessory_module.accessory_list[part as usize]) >> 8;
                let item = ITEM_INDEX_MASK & self.accessory_module.accessory_list[part as usize];
                log += &format!("{:?} - lv: {}, idx: {}\n", part, lv, item);
            }
        }
        log
    }

    pub fn extract_data(&self, config: &GameplayConfigManager) -> CharacterLogicData {
        // Character init value for testing
        let test_hp_remain_rate = config.get_char_game_init_hp_rate();
        let test_cd_filled_rate = config.get_char_game_init_cd_rate();
        let current_hp = (self.attribute.get_max_hp() as f64 * test_hp_remain_rate as f64 / 100.0)
            .round() as u32;
        let mut skill = self.attribute.get_skill_meta().clone();
        skill.set_testing_init_skill_cool_down(test_cd_filled_rate);

        CharacterLogicData {
            id: *self.attribute.get_id(),
            max_hp: self.attribute.get_max_hp(),
            current_hp,
            atk: self.attribute.get_atk(),
            def: self.attribute.get_def(),
            element: self.attribute.get_element(),
            special_tile: self.attribute.get_special_tile().clone(),
            skill,
            passive: self.attribute.get_passive().clone(),
            buff_states: vec![],
            assist_nerf_modifier: self.attribute.get_assist_nerf_modifier(),
        }
    }

    pub fn get_id(&self) -> &Uuid {
        self.attribute.get_id()
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, EnumString)]
pub enum EnemyType {
    #[strum(serialize = "normal", serialize = "n")]
    Normal,
    #[strum(serialize = "elite", serialize = "e")]
    Elite,
    #[strum(serialize = "boss", serialize = "b")]
    Boss,
}

impl Default for EnemyType {
    fn default() -> Self {
        EnemyType::Normal
    }
}

impl std::fmt::Display for EnemyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnemyType::Normal => write!(f, "{}", NORMAL_ENEMY_STRING),
            EnemyType::Elite => write!(f, "{}", ELITE_ENEMY_STRING),
            EnemyType::Boss => write!(f, "{}", BOSS_ENEMY_STRING),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub enum CommandType {
    Attack,
    Skill,
    Random,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub enum AttackDecision {
    Random,
    LowestHp,
    BenefitElement,
    //OptimizeDamage
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Command {
    pub command_type: CommandType,
    pub skill_info: Option<SkillInfo>, // `None` if command type is not `CommandType::Skill`
    pub attack_decision: AttackDecision,
}

impl Default for Command {
    fn default() -> Self {
        Self {
            command_type: CommandType::Attack,
            skill_info: None,
            attack_decision: AttackDecision::BenefitElement,
        }
    }
}

impl Command {
    pub fn is_attack_action(&self) -> bool {
        match self.command_type {
            CommandType::Attack => true,
            CommandType::Skill => match self.skill_info {
                Some(SkillInfo::NpcAttack)
                | Some(SkillInfo::Damage)
                | Some(SkillInfo::LineEliminate)
                | Some(SkillInfo::ElementalExplosion) => true,
                _ => false,
            },
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnemyScriptMap {
    pub script_map: HashMap<String, Vec<Command>>, // script_name => script commands
}

impl EnemyScriptMap {
    pub fn get_command(&self, script_name: &str, turn: usize) -> Result<Command, GameError> {
        let script = self
            .script_map
            .get(script_name)
            .ok_or(GameError::EnemyScriptNotFound(script_name.to_owned()))?;

        let command = &script[(turn / 2 - 1) % script.len()];
        log::debug!("\nEnemy Command Raw: {:#?}", command);

        let result_command = match command.command_type {
            CommandType::Random => {
                let skill_info = SkillInfo::from(
                    // The random method will be replace in the future. Not optimized for now.
                    rand::thread_rng().gen_range(SkillInfo::random_enemy_command_range()),
                );
                let ran_command = if skill_info == SkillInfo::NpcAttack {
                    Command {
                        command_type: CommandType::Attack,
                        skill_info: None,
                        attack_decision: command.attack_decision,
                    }
                } else {
                    Command {
                        command_type: CommandType::Skill,
                        skill_info: Some(skill_info),
                        attack_decision: command.attack_decision,
                    }
                };
                log::debug!("\nRolled Command: {:#?}", ran_command);
                ran_command
            }
            _ => *command,
        };

        Ok(result_command)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnemyScript {
    pub name: String,
    pub commands: Vec<Command>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnemyTemplateRequest {
    enemy_template_name: String,
    comment: Option<String>,
    enemy_attr: EnemyAttribute,
    enemy_script_name: String,
}

impl EnemyTemplateRequest {
    pub fn is_valid(&self) -> bool {
        !self.enemy_template_name.is_empty() && self.enemy_attr.is_valid_param()
    }

    pub fn get_enemy_attr(&self) -> &EnemyAttribute {
        &self.enemy_attr
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnemyTemplate {
    pub enemy_template_name: String,
    pub comment: String,
    pub char_config: GameplayConfigManager,
    pub enemy_script_name: String,
    pub enemy_type: EnemyType,    // Currently not being used
    pub lift_rate: EnemyLiftRate, // Will be used while implement the dungeon difficulty feature
}

impl Default for EnemyTemplate {
    fn default() -> Self {
        Self::compose_random_template(Default::default())
    }
}

impl EnemyTemplate {
    pub fn new(template_req: &EnemyTemplateRequest, enemy_config: GameplayConfigManager) -> Self {
        Self {
            enemy_template_name: template_req.enemy_template_name.clone(),
            comment: template_req
                .comment
                .clone()
                .unwrap_or_else(|| String::new()),
            char_config: enemy_config,
            enemy_script_name: template_req.enemy_script_name.clone(),
            enemy_type: template_req.enemy_attr.enemy_type.clone(),
            lift_rate: template_req.enemy_attr.lift_rate.clone(),
        }
    }

    pub fn compose_random_template(enemy_type: EnemyType) -> Self {
        let random_template_name = format!("random_{}_template_{}", enemy_type, Uuid::new_v4());
        log::debug!("Template name: {}", random_template_name);

        Self {
            enemy_template_name: random_template_name,
            comment: String::default(),
            char_config: GameplayConfigManager::new(),
            enemy_script_name: String::default(),
            enemy_type,
            lift_rate: EnemyLiftRate::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnemyAttribute {
    pub enemy_type: EnemyType,
    pub hp_min: u32,
    pub hp_max: u32,
    pub atk_min: u32,
    pub atk_max: u32,
    pub def_min: u32,
    pub def_max: u32,
    pub lift_rate: EnemyLiftRate,
}

impl EnemyAttribute {
    pub fn is_valid_param(&self) -> bool {
        self.hp_min < self.hp_max && self.def_min < self.def_max && self.atk_min < self.atk_max
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnemyLiftRate {
    pub hp: f64,
    pub atk: f64,
    pub def: f64,
}

impl Default for EnemyLiftRate {
    fn default() -> Self {
        // Temp value
        EnemyLiftRate {
            hp: 0.1,
            atk: 0.1,
            def: 0.1,
        }
    }
}
