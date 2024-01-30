use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::EnumCount;
use strum_macros::{EnumCount as EnumCountMacro, EnumIter};

use super::art_assets_count::ART_ASSET_AMOUNT;
use crate::game_core::probability_mod::*;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, EnumCountMacro, EnumIter, PartialEq)]
pub enum BaseRace {
    Pecora,  // deer, cow, goat, antelope, sheep
    Aves,    // birds, duck, eagle, parrot
    Felidae, // cat, tiger, lion, leopard
    Canidae, // dog, wolf, fox
}

impl Default for BaseRace {
    fn default() -> Self {
        Self::Canidae
    }
}

impl From<u32> for BaseRace {
    fn from(i: u32) -> Self {
        match i {
            0 => Self::Pecora,
            1 => Self::Aves,
            2 => Self::Felidae,
            3 => Self::Canidae,
            _ => unreachable!(),
        }
    }
}

impl FromStr for BaseRace {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pecora" | "Pecora" => Ok(Self::Pecora),
            "aves" | "Aves" => Ok(Self::Aves),
            "felidae" | "Felidae" => Ok(Self::Felidae),
            "canidae" | "Canidae" => Ok(Self::Canidae),
            _ => Err(format!(
                "[ERROR] \"{}\" is not a valid value of BaseRace.",
                s
            )),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, EnumCountMacro)]
pub enum BodyPart {
    Head,
    Mouth,
    EarHorn,
    Body,
    Arm,
    Leg,
    Tail,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OrganModule {
    pub race: BaseRace,
    pub head: u8,
    pub mouth: u8,    // Has and individual color for Aves
    pub ear_horn: u8, // Has and individual color for Pecora
    pub body: u8,
    pub arm_l: u8,
    pub arm_r: u8,
    pub leg_l: u8,
    pub leg_r: u8,
    pub tail: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BaseBodyModule {
    pub style: OrganModule,
    pub main_color: Color,
    pub special_color: Color, // Aves and Pecora may have an individual color body part
}

impl BaseBodyModule {
    pub fn roll_base_body_module() -> BaseBodyModule {
        let mut rand_holder = RANDOM_NUM_HOLDER.write().expect(LOCK_POISONED);

        // Roll race
        // ### MEMO: roll race first may cause not uniform probability of all type of body module
        let race = BaseRace::from(rand_holder.sample(..BaseRace::COUNT as u32));

        // Roll body set
        // ### MEMO: currently only 1 style set png done, so based on body's max index as the max value.
        let base_body_style_set = rand_holder
            .sample(1..=ART_ASSET_AMOUNT.base_body[BodyPart::Body as usize][race as usize])
            as u8;

        // Roll color
        let main_color = Color {
            r: rand_holder.sample(..=u8::MAX as u32) as u8,
            g: rand_holder.sample(..=u8::MAX as u32) as u8,
            b: rand_holder.sample(..=u8::MAX as u32) as u8,
        };

        // If race is Aves or Pecora, roll another special color for unique body part
        let special_color = if race == BaseRace::Aves || race == BaseRace::Pecora {
            // Reroll a new color
            Color {
                r: rand_holder.sample(..=u8::MAX as u32) as u8,
                g: rand_holder.sample(..=u8::MAX as u32) as u8,
                b: rand_holder.sample(..=u8::MAX as u32) as u8,
            }
        } else {
            // Follow the main color
            main_color.clone()
        };

        /*
        log::debug!(
            "   ### base_body used_bit:{}, rand_consumed: {}",
            rand_holder.bit_consumed,
            rand_holder.rand_consumed
        );
        */

        BaseBodyModule {
            style: OrganModule {
                race,
                head: base_body_style_set,
                mouth: base_body_style_set,
                ear_horn: base_body_style_set,
                body: base_body_style_set,
                arm_l: base_body_style_set,
                arm_r: base_body_style_set,
                leg_l: base_body_style_set,
                leg_r: base_body_style_set,
                tail: base_body_style_set,
            },
            main_color,
            special_color,
        }
    }
}
