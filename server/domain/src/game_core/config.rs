use atb_types::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Error;
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{EnumCount, EnumIter, EnumString};

use super::lazy_static;
use crate::game_core::character::EnemyAttribute;
use crate::game_core::character_mod::char_const::get_default_char_attr_config;
use crate::game_core::{GameError, ServerError};

lazy_static::lazy_static! {
    static ref DEFAULT_ELEM_BASE_INFO_CONFIG: ElementBaseInfoConfig = serde_json::from_slice(include_bytes!("./config/element_base_info.json")).expect("can't not parse element_base_info setting config");
    static ref DEFAULT_ENERGY_CHARGE_INFO_CONFIG: EnergyChargeInfo = serde_json::from_slice(include_bytes!("./config/energy_charge_info.json")).expect("can't not parse energy_charge_info.json setting config");
    static ref DEFAULT_DAMAGE_FORMULA_COEF_CONFIG: DamageFormulaCoefficient = serde_json::from_slice(include_bytes!("./config/damage_formula_coef.json")).expect("can't not parse damage_formula_coef.json setting config");
    static ref DEFAULT_CHAR_GAME_INIT_STATUS_CONFIG: CharGameInitStatus = serde_json::from_slice(include_bytes!("./config/char_game_init_status.json")).expect("can't not parse char_game_init_status.json setting config");

}

// Temporary ID
pub const ENEMY_ADDR: &str = "enemy";
pub const TUTORIAL_RIVAL_ADDR: &str = "tutorial_rival";

pub const RATE_UNIT: u32 = 1_000;

//#TODO: These config should be fed from external config!
//0.001ETH
pub const STAKE: &str = "1000000000000000";
//0.005ETH
// pub const WITHDRAWAL_FEE: &str = "5000000000000000";
pub const ADMIN_WALLET_ADDRESS: &str = "0x2Af645839ea4ca82452aFd195e210420e7Cc1F90";

pub const DEFAULT_INGAME_CURRENCY: u32 = 0;
pub const CURRENCY_DECAY_RATE: f64 = 0.1;
pub const CURRENCY_REWARD_BASE: u32 = 100;
pub const CURRENCY_REWARD_PVE: u32 = 1000;

pub const MAX_PARTY_MEMBER: usize = 3;
pub const MAX_ENEMY_MEMBER: usize = 1;

pub const BOARD_NUM_COLORS: u32 = Bead::COUNT as u32;
pub const BOARD_WIDTH: u32 = 8;
pub const BOARD_HEIGHT: u32 = 7;
pub const PRIVATE_CODE_LENGTH: usize = 6;

pub const MAX_ZONE_RECORD_SIZE: usize = 2;

pub const ROUND_DECAY_THRESHOLD: u32 = 10; // Less than round threshold will remain original score
pub const ROUND_CAP: u32 = 30; // Exceed round cap will only get 1 point

pub const DEFAULT_ZONE_BUFF_RATE: u32 = 5000;
pub const MAX_ZONE_BUFF_RATE: u32 = 10000;

pub const DEFAULT_ZONE_EXPIRED_TURN: u8 = 255;

pub const ELO_INIT_SCORE: u32 = 1200;

pub const DEFAULT_DUNGEON_NAME: &str = "default";
pub const DEFAULT_ENEMY_TEMPLATE_NAME: &str = "default";
pub const DEFAULT_ENEMY_SCRIPT_NAME: &str = "default";
pub const DEFAULT_DUNGEON_STAGE_NUMBER: usize = 500;

pub const NORMAL_ENEMY_STRING: &str = "normal";
pub const ELITE_ENEMY_STRING: &str = "elite";
pub const BOSS_ENEMY_STRING: &str = "boss";

pub const DEFAULT_RIFT_LEVEL: u32 = 0;
pub const DEFAULT_STAGE_LEVEL: u32 = 0;

#[cfg(feature = "debug_tool")]
pub const TEST_BOARD_PATH: &str = "./domain/src/game_core/test_board/";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameplayConfigManager {
    config_info: ConfigInfo,

    // Runtime initialized fields, evaluated by config_info
    tier_boundary_config: TierBoundaryConfig,
    element_modifier: HashMap<Element, Vec<u32>>,
}

impl GameplayConfigManager {
    pub fn new() -> Self {
        let config_info = ConfigInfo::new();
        let tier_boundary_config = TierBoundaryConfig::new(&config_info.char_attr_config);
        let element_modifier = Self::init_element_modifier(&config_info.game_scene_env_config);

        Self {
            config_info,
            tier_boundary_config,
            element_modifier,
        }
    }

    fn init_element_modifier(
        game_scene_env_config: &GameSceneEnvConfig,
    ) -> HashMap<Element, Vec<u32>> {
        // Element modifier is a weighting factor used to describe the relative strength of attacks between each element.
        // It is represented by a Vec<u32> of 5 values for each element,.
        // Where the values in the array represent the weighted values of the element against the other 5 elements.
        // E.g. Fire -> [1,000(Fire), 1,200(Wind), 800(Water), 1,000(Light), 1,000(Shadow)]
        let mut modifier = HashMap::<Element, Vec<u32>>::new();
        for elem in Element::iter() {
            if elem == Element::Unknown {
                break;
            }
            let elem_info = DEFAULT_ELEM_BASE_INFO_CONFIG.elements.get(&elem).unwrap();
            let mut counter_list = vec![RATE_UNIT; 5];
            counter_list[elem_info.advantage_elem as usize] =
                game_scene_env_config.elem_advantage_rate;
            counter_list[elem_info.disadvantage_elem as usize] =
                game_scene_env_config.elem_weakness_rate;

            modifier.insert(elem, counter_list);
        }
        modifier
    }

    fn get_element_modifier(&self, attacker_element: &Element) -> Result<&Vec<u32>, Error> {
        self.element_modifier.get(attacker_element).ok_or(Error)
    }

    pub fn apply_element_modifier(
        &self,
        attacker_produced_damage: u32,
        attacker_element: &Element,
        defender_element: &Element,
    ) -> Result<i32, Error> {
        let element_modifier = self.get_element_modifier(&attacker_element)?;

        let defender_received_damage =
            attacker_produced_damage * element_modifier[*defender_element as usize] / RATE_UNIT;

        log::debug!(
            "   element_modifier = {:.1}x",
            element_modifier[*defender_element as usize] as f32 / RATE_UNIT as f32
        );

        Ok(defender_received_damage as i32)
    }

    pub fn is_valid_param(&self) -> bool {
        self.config_info.is_valid_param()
    }

    pub fn apply_custom_config(&mut self, custom_config: &ConfigInfo) -> Result<(), ServerError> {
        if !custom_config.is_valid_param() {
            return Err(ServerError::InvalidConfigParam);
        }
        self.config_info = custom_config.clone();
        self.tier_boundary_config = TierBoundaryConfig::new(&custom_config.char_attr_config);
        Ok(())
    }

    pub fn show_config(&self) {
        //###TODO: will be serialize to json string for console command
        log::debug!("{:#?}", self);
    }

    pub fn show_config_name(&self) -> &str {
        &self.config_info.config_name
    }

    pub fn get_config_info(&self) -> &ConfigInfo {
        &self.config_info
    }

    pub fn get_rounds_decay_param(&self) -> (u32, u32) {
        (
            self.config_info.game_scene_env_config.round_decay_threshold,
            self.config_info.game_scene_env_config.round_cap,
        )
    }

    pub fn get_char_attr_config(&self) -> &CharacterBasicAttributeConfig {
        &self.config_info.char_attr_config
    }

    pub fn get_assist_modifier_rate(&self) -> u32 {
        self.config_info.char_attr_config.assist_modifier_rate
    }

    pub fn get_tier_range(&self, k: TieredType) -> &TierRange {
        match k {
            TieredType::HP => &self.tier_boundary_config.hp,
            TieredType::ATK => &self.tier_boundary_config.atk,
            TieredType::DEF => &self.tier_boundary_config.def,
            TieredType::MONO_SP_GEM => &self.tier_boundary_config.mono_sp_gem,
        }
    }

    pub fn get_damage_formula_coef(&self) -> &DamageFormulaCoefficient {
        &self.config_info.game_scene_env_config.damage_formula
    }

    pub fn get_charge_info(&self) -> &EnergyChargeInfo {
        &self.config_info.game_scene_env_config.energy_charge_info
    }

    pub fn get_auto_charge_each_turn(&self) -> u32 {
        self.config_info
            .game_scene_env_config
            .energy_charge_info
            .recovery_each_turn
    }

    pub fn get_char_game_init_hp_rate(&self) -> u32 {
        self.config_info.char_game_init_status.hp_remain_rate
    }

    pub fn get_char_game_init_cd_rate(&self) -> u32 {
        self.config_info.char_game_init_status.cd_filled_rate
    }

    pub fn get_zone_buff_rate(&self) -> u32 {
        self.config_info.game_scene_env_config.zone_buff_rate
    }

    pub fn get_zone_expired_turn(&self) -> u8 {
        self.config_info
            .game_scene_env_config
            .zone_effect_expired_turn
    }

    pub fn set_char_game_init_cd_rate(&mut self, percent: u32) {
        self.config_info.char_game_init_status.cd_filled_rate = percent;
    }

    pub fn overwrite_enemy_config(&mut self, enemy_attr: &EnemyAttribute) {
        self.config_info.overwrite_enemy_config(enemy_attr);
        self.tier_boundary_config = TierBoundaryConfig::new(&self.config_info.char_attr_config)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigInfo {
    config_name: String,
    comment: String,
    char_attr_config: CharacterBasicAttributeConfig,
    game_scene_env_config: GameSceneEnvConfig,
    char_game_init_status: CharGameInitStatus,
}

impl ConfigInfo {
    pub fn new() -> Self {
        Self {
            config_name: "Server Default".to_string(),
            comment: "Config from server default".to_string(),
            char_attr_config: CharacterBasicAttributeConfig::new(),
            game_scene_env_config: GameSceneEnvConfig::new(),
            char_game_init_status: CharGameInitStatus::new(),
        }
    }

    pub fn is_valid_param(&self) -> bool {
        self.char_attr_config.is_valid_param() && self.game_scene_env_config.is_valid_param()
    }

    fn overwrite_enemy_config(&mut self, enemy_attr: &EnemyAttribute) {
        self.char_attr_config.overwrite_enemy_config(enemy_attr)
    }
}

// Tier config data will later evaluated at runtime
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TierBoundaryConfig {
    pub hp: TierRange,
    pub atk: TierRange,
    pub def: TierRange,
    pub mono_sp_gem: TierRange,
}

impl TierBoundaryConfig {
    pub fn new(attr_config: &CharacterBasicAttributeConfig) -> Self {
        Self {
            hp: TierRange::new(attr_config.hp_min, attr_config.hp_max),
            atk: TierRange::new(attr_config.atk_min, attr_config.atk_max),
            def: TierRange::new(attr_config.def_min, attr_config.def_max),
            mono_sp_gem: TierRange::new(attr_config.mono_sp_gem_min, attr_config.mono_sp_gem_max),
        }
    }
}

// Tier related value range: Tier[0(Premium), 1, 2, 3, 4(evenly)] (tier_lv 4 is evenly test level)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TierRange {
    pub tier_min: [u32; 5],
    pub tier_max: [u32; 5],
}

impl TierRange {
    pub fn new(min: u32, max: u32) -> Self {
        // TODO: This is a temporary value be able to normally run the game, needs design new rule to handle this.
        Self {
            tier_min: [min, min, min, min, min],
            tier_max: [max, max, max, max, max],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(non_camel_case_types)]
pub enum TieredType {
    HP,
    ATK,
    DEF,
    MONO_SP_GEM,
}

// Custom config API struct
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CharacterBasicAttributeConfig {
    pub hp_min: u32,
    pub hp_max: u32,
    pub def_min: u32,
    pub def_max: u32,
    pub atk_min: u32,
    pub atk_max: u32,
    pub mono_sp_gem_min: u32,
    pub mono_sp_gem_max: u32,
    pub dual_sp_gem_min: i32,
    pub dual_sp_gem_range: u32,
    pub dual_sp_gem_gap_start: i32,
    pub dual_sp_gem_gap_range: u32,
    pub assist_modifier_rate: u32,
}

impl CharacterBasicAttributeConfig {
    pub fn new() -> Self {
        get_default_char_attr_config()
    }

    // "min" must less than and not equal to "max", "range" can not be 0.
    fn is_valid_param(&self) -> bool {
        self.hp_min < self.hp_max
            && self.def_min < self.def_max
            && self.atk_min < self.atk_max
            && self.mono_sp_gem_min < self.mono_sp_gem_max
            && self.dual_sp_gem_min < self.dual_sp_gem_gap_start
            && self.dual_sp_gem_range != 0
            && self.dual_sp_gem_gap_range != 0
    }

    fn overwrite_enemy_config(&mut self, enemy_attr: &EnemyAttribute) {
        self.hp_min = enemy_attr.hp_min;
        self.hp_max = enemy_attr.hp_max;
        self.atk_min = enemy_attr.atk_min;
        self.atk_max = enemy_attr.atk_max;
        self.def_min = enemy_attr.def_min;
        self.def_max = enemy_attr.def_max;
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GameSceneEnvConfig {
    pub elem_advantage_rate: u32,
    pub elem_weakness_rate: u32,
    pub zone_buff_rate: u32,
    pub zone_effect_expired_turn: u8,
    pub damage_formula: DamageFormulaCoefficient,
    pub energy_charge_info: EnergyChargeInfo,
    pub round_decay_threshold: u32,
    pub round_cap: u32,
}

impl GameSceneEnvConfig {
    pub fn new() -> Self {
        let elem_info_vale = &DEFAULT_ELEM_BASE_INFO_CONFIG.value;
        Self {
            elem_advantage_rate: elem_info_vale.advantage_rate,
            elem_weakness_rate: elem_info_vale.weakness_rate,
            zone_buff_rate: DEFAULT_ZONE_BUFF_RATE,
            zone_effect_expired_turn: DEFAULT_ZONE_EXPIRED_TURN,
            damage_formula: DamageFormulaCoefficient::new(),
            energy_charge_info: EnergyChargeInfo::new(),
            round_decay_threshold: ROUND_DECAY_THRESHOLD,
            round_cap: ROUND_CAP,
        }
    }

    fn is_valid_param(&self) -> bool {
        self.damage_formula.is_valid_param()
            && self.energy_charge_info.is_valid_param()
            && self.round_cap > 0
            && self.round_cap > self.round_decay_threshold
            && self.zone_buff_rate <= MAX_ZONE_BUFF_RATE
            && self.zone_effect_expired_turn > 0
    }
}

// About the details of each variables' meaning in the formula, please refer to the wiki page: "Damage Formula"
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DamageFormulaCoefficient {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub exp: u32,
    pub decrease_rate: f64,
}

impl DamageFormulaCoefficient {
    fn new() -> Self {
        DEFAULT_DAMAGE_FORMULA_COEF_CONFIG.clone()
    }

    fn is_valid_param(&self) -> bool {
        self.a > 0.0
            && self.b > 0.0
            && self.c > 0.0
            && self.d > 0.0
            && self.decrease_rate >= 0.0
            && self.decrease_rate < 1.0
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnergyChargeInfo {
    pub clear_unit: u32,
    pub self_color_multiply: f64,
    pub diff_color_multiply: f64,
    pub recovery_each_turn: u32,

    pub recover_by_get_hit: GetHitRecoveryType,
    // Following 2 fields are avaliable while "recover_by_get_hit" assigned particular field
    pub fixed_recovery_amount_by_hit: u32,
    pub recovery_by_damage_rate: f64,
}

impl EnergyChargeInfo {
    pub fn new() -> Self {
        DEFAULT_ENERGY_CHARGE_INFO_CONFIG.clone()
    }

    fn is_valid_param(&self) -> bool {
        self.self_color_multiply >= 0.0
            && self.diff_color_multiply >= 0.0
            && self.fixed_recovery_amount_by_hit > 0
            && self.recovery_by_damage_rate >= 0.0
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum GetHitRecoveryType {
    None,
    Fixed,
    DamageRate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CharGameInitStatus {
    pub hp_remain_rate: u32,
    pub cd_filled_rate: u32,
}

impl CharGameInitStatus {
    pub fn new() -> Self {
        DEFAULT_CHAR_GAME_INIT_STATUS_CONFIG.clone()
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, EnumIter, EnumCount)]
pub enum Bead {
    Red,
    Green,
    Blue,
    Yellow,
    Purple,
}

impl From<Element> for Bead {
    fn from(e: Element) -> Self {
        match e {
            Element::Fire => Bead::Red,
            Element::Wind => Bead::Green,
            Element::Water => Bead::Blue,
            Element::Light => Bead::Yellow,
            Element::Shadow => Bead::Purple,
            _ => unreachable!("mapping bead failed"),
        }
    }
}

#[derive(
    Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash, EnumIter, EnumCount, EnumString,
)]
pub enum Element {
    #[strum(serialize = "fire")]
    Fire,
    #[strum(serialize = "wind")]
    Wind,
    #[strum(serialize = "water")]
    Water,
    #[strum(serialize = "light")]
    Light,
    #[strum(serialize = "shadow")]
    Shadow,
    Unknown,
}

impl Default for Element {
    fn default() -> Self {
        Self::Unknown
    }
}

impl From<u32> for Element {
    fn from(v: u32) -> Self {
        match v {
            0 => Self::Fire,
            1 => Self::Wind,
            2 => Self::Water,
            3 => Self::Light,
            4 => Self::Shadow,
            _ => unreachable!(),
        }
    }
}

impl Element {
    fn get_weakness_info(&self) -> Result<&WeaknessInfo, GameError> {
        DEFAULT_ELEM_BASE_INFO_CONFIG
            .elements
            .get(&self)
            .ok_or(GameError::CharacterElementError)
    }

    pub fn get_counter_bead(&self) -> Result<Bead, GameError> {
        let element_info = self.get_weakness_info()?;
        let counter_bead = Bead::from(element_info.disadvantage_elem);
        Ok(counter_bead)
    }

    pub fn get_advantage_element(&self) -> Result<Element, GameError> {
        Ok(self.get_weakness_info()?.advantage_elem)
    }

    pub fn get_disadvantage_element(&self) -> Result<Element, GameError> {
        Ok(self.get_weakness_info()?.disadvantage_elem)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, EnumCount, EnumString)]
pub enum ClearPattern {
    #[strum(serialize = "free")]
    Free,
    #[strum(serialize = "vertical")]
    Vertical,
    #[strum(serialize = "horizontal")]
    Horizontal,
}

impl From<u32> for ClearPattern {
    fn from(i: u32) -> Self {
        match i {
            0 => Self::Free,
            1 => Self::Vertical,
            2 => Self::Horizontal,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ElementBaseInfoConfig {
    pub elements: HashMap<Element, WeaknessInfo>,
    pub value: WeaknessValue,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WeaknessInfo {
    bead: Bead,
    advantage_elem: Element,
    disadvantage_elem: Element,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WeaknessValue {
    advantage_rate: u32,
    weakness_rate: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum DamageSource {
    MainAttacker,
    AssistAttacker,
    SkillDamage,
    SkillRecovery,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DamageResult {
    pub damage_source: DamageSource,
    pub attacker: Uuid,
    pub defender: Uuid,
    pub attacker_produced_damage: u32,
    pub defender_received_damage: i32,
    pub shield_blocking: bool,
}

pub fn is_real_player_addr(user_addr: &str) -> bool {
    !(user_addr == ENEMY_ADDR || user_addr == TUTORIAL_RIVAL_ADDR)
}

pub enum DungeonGamer {
    Player,
    Enemy,
}

/// Testing feature
#[cfg(feature = "debug_tool")]
pub fn fetch_test_board_name() -> Result<Vec<String>, ServerError> {
    use std::fs;
    let paths = fs::read_dir(TEST_BOARD_PATH).or_else(|_| Err(ServerError::InvalidFilePath))?;
    let file_names = paths
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str().map(|s| String::from(s)))
            })
        })
        .collect::<Vec<String>>();

    log::debug!("   test board list: {:?}", file_names);
    Ok(file_names)
}
