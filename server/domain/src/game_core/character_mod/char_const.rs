use crate::game_core::config::CharacterBasicAttributeConfig;
use atb::prelude::*;

lazy_static::lazy_static! {
    pub static ref DEFAULT_CHAR_ATTR_CONFIG: CharacterBasicAttributeConfig = serde_json::from_slice(include_bytes!("../config/default_char_attributes.json")).expect("can't not parse DEFAULT_CHAR_ATTR_CONFIG");
}

// Global setting
pub const PREMIUM_BASE_RATE: u32 = 0_750;

pub const MAX_RARITY_LV: usize = 5;

pub const FREE_NFT_LOWEST_TIER_LV: usize = 3;
pub const EVEN_CHANCE_TIER_LV: usize = FREE_NFT_LOWEST_TIER_LV + 1;

pub const MAX_ACCESSORY_INDEX: u32 = 3;
pub const MAX_BODY_MODULE_COUNT: u8 = 3;

// Rarity score related
pub const RARITY_SLOT: u8 = 10;
pub const RARITY_NO_SPC_SCORE: u8 = 5;
pub const RARITY_DUAL_SPC_SCORE: u8 = 10;

pub fn get_default_char_attr_config() -> CharacterBasicAttributeConfig {
    DEFAULT_CHAR_ATTR_CONFIG.clone()
}
