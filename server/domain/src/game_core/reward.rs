use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

use super::character::CharacterV2;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Reward {
    pub winner_reward: String,
    pub acquire_new_character: bool,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct RewardCache {
    pub reward_types: Vec<RewardType>,
    pub character_rewards: Vec<CharacterReward>,
    pub currency_rewards: Vec<CurrencyReward>,
    pub character_rewards_index: HashMap<usize, usize>, // index of reward_types -> index of character_rewards
    pub currency_rewards_index: HashMap<usize, usize>, // index of reward_types -> index of currency_rewards
}

#[derive(Debug, Clone, Deserialize, Serialize, EnumString, PartialEq, Eq)]
pub enum RewardType {
    #[strum(serialize = "consolation")]
    Consolation,
    #[strum(serialize = "ingame_currency")]
    IngameCurrency,
    #[strum(serialize = "character")]
    Character,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterReward {
    pub char_data: CharacterV2,
    pub cost: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CurrencyReward {
    pub amount: u32,
}
