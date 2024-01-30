use atb::prelude::*;
use serde::{Deserialize, Serialize};
use strum::EnumCount;
use strum_macros::{EnumCount as EnumCountMacro, EnumIter, EnumString};

use super::base_body_module::BaseRace;
use super::char_const::*;

lazy_static::lazy_static! {
    pub static ref ART_ASSET_AMOUNT: ArtAssetAmount = serde_json::from_slice(include_bytes!("../config/asset_amount.json")).expect("can't not parse ART_ASSET_AMOUNT config");
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, EnumIter, EnumCountMacro, EnumString)]
pub enum AccPartFileName {
    head,
    face,
    neck,
    bodyArmor,
    waistArmor,
    arm,
    foot,
    eye,
    weapon,
    sidearms,
    floatingItem,
    groundItem,
    groundEffect,
    backgroundEffect,
}

impl From<usize> for AccPartFileName {
    fn from(u: usize) -> Self {
        match u {
            0 => Self::head,
            1 => Self::face,
            2 => Self::neck,
            3 => Self::bodyArmor,
            4 => Self::waistArmor,
            5 => Self::arm,
            6 => Self::foot,
            7 => Self::eye,
            8 => Self::weapon,
            9 => Self::sidearms,
            10 => Self::floatingItem,
            11 => Self::groundItem,
            12 => Self::groundEffect,
            13 => Self::backgroundEffect,
            _ => unreachable!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, EnumIter, EnumCountMacro, EnumString)]
pub enum BodyPartFileName {
    head,
    mouth,
    ear, // In original file name is "ear_horn", remove the "_horn" substring to avoiding unnecessary "_".
    body,
    arm,
    leg,
    tail,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArtAssetAmount {
    pub accessory: [[u32; (MAX_RARITY_LV + 1) as usize]; AccPartFileName::COUNT], //accessory[part][lv]
    pub base_body: [[u32; BaseRace::COUNT]; BodyPartFileName::COUNT], //base_body[part][race]
}
