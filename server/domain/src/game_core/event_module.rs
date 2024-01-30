use super::{config::ClearPattern, lazy_static};
//use atb::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::game_core::config::Element;

lazy_static::lazy_static! {
    pub static ref EVENT_CONDITION_CONFIG: EventConditionConfig = serde_json::from_slice(include_bytes!("./config/event_condition.json")).expect("can't not parse EVENT_CONDITION_CONFIG config");
    pub static ref EVENT_INFO_CONFIG: EventInfoConfig = serde_json::from_slice(include_bytes!("./config/event_info_table.json")).expect("can't not parse EVENT_INFO_CONFIG config");
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Hash, Eq, PartialEq)]
pub enum EventName {
    ExtremelyHot,
    SuperWindy,
    FreezingCold,
    SoBright,
    DarkNight,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum EventTriggerType {
    Consecutive,
    Accumulate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum EventEffectType {
    Multiple,
    Additive,
}

// ### TODO: Nameing and structure are temporary, could be rename and simplfy
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GamerMove {
    pub elem: Element,
    pub clear_pattern: ClearPattern,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EventConditionConfig {
    event_condition_list: Vec<EventCondition>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EventCondition {
    name: EventName,
    elem: Element,
    trigger_type: EventTriggerType,
    clear_pattern: ClearPattern,
    trigger_amount: u32,
}

impl EventCondition {
    fn is_match(&self, move_buffer: &[GamerMove]) -> bool {
        match self.trigger_type {
            EventTriggerType::Consecutive => self.is_consecutive_match(move_buffer),
            EventTriggerType::Accumulate => self.is_accumulate_match(move_buffer),
        }
    }

    fn is_consecutive_match(&self, move_buffer: &[GamerMove]) -> bool {
        // Check whether the buffer last is valid
        if !move_buffer
            .last()
            .map_or(false, |last| last.elem == self.elem)
        {
            return false;
        }

        // Check is the same element consecutive in buffer (count from last element)
        let consecutive_count = move_buffer
            .iter()
            .rev()
            .take_while(|m| m.elem == self.elem)
            .count() as u32;

        log::debug!(
            "    Consecutive [{:?}] count: {}",
            self.elem,
            consecutive_count
        );

        consecutive_count == self.trigger_amount
    }

    fn is_accumulate_match(&self, move_buffer: &[GamerMove]) -> bool {
        let accumulate_count =
            move_buffer
                .iter()
                .fold(0, |acc, m| if m.elem == self.elem { acc + 1 } else { acc })
                as u32;

        log::debug!(
            "    Accumulate [{:?}] count: {}",
            self.elem,
            accumulate_count
        );

        accumulate_count == self.trigger_amount
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EventInfoConfig {
    event_info_table: HashMap<EventName, LazyInitEventInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LazyInitEventInfo {
    pub target_elem: Element,
    pub effect_type: EventEffectType,
}

// ### TODO: This struct or naming might change if new effect SPEC in the future.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventInfo {
    pub target_elem: Element,
    #[serde(skip, default = "default_effect_type")]
    pub effect_type: EventEffectType,
}

fn default_effect_type() -> EventEffectType {
    EventEffectType::Multiple
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameEvent {
    pub name: EventName,
    pub info: EventInfo,
}

impl GameEvent {
    pub fn is_valid(&self, current_turn: u8, config_expired_turn: u8) -> bool {
        current_turn <= config_expired_turn
    }
}

pub fn update_event(
    current_event: &Option<GameEvent>,
    move_buffer: &mut Vec<GamerMove>,
    current_turn: u8,
    config_expired_turn: u8,
) -> Option<GameEvent> {
    // Check is new event triggered and setting the GameEvent
    let mut next_event = match_new_event(move_buffer)
        .and_then(|name| get_event_info(&name).map(|info| GameEvent { name, info }));

    if next_event.is_none() {
        // If no new event triggered, apply previous event if it is not expired
        if let Some(e) = current_event {
            if e.is_valid(current_turn, config_expired_turn) {
                next_event = current_event.clone();
            }
        }
    }

    next_event
}

fn match_new_event(move_buffer: &[GamerMove]) -> Option<EventName> {
    // The condition list is a priority list
    // If a prior condition matched, it will stop and return the result.
    for condition in &EVENT_CONDITION_CONFIG.event_condition_list {
        if condition.is_match(&move_buffer) {
            log::debug!("   ### Match and trigger new event [{:?}]", condition.name);
            return Some(condition.name);
        }
    }
    None
}

fn get_event_info(event_name: &EventName) -> Option<EventInfo> {
    // Due to the requirement for serialization consistency of fields in the Cartesi mode, some fields must be skipped in EventInfo.
    // However, skipping these fields makes lazy_static unable to directly serialize and init structure.
    // So another structure(LazyInitEventInfo) is used to first initialize the config and then copy it to the desired structure.
    EVENT_INFO_CONFIG
        .event_info_table
        .get(event_name)
        .cloned()
        .map(|lazy_init| EventInfo {
            target_elem: lazy_init.target_elem,
            effect_type: lazy_init.effect_type,
        })
}
