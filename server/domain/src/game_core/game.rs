use atb_types::prelude::uuid::Uuid;
use atb_types::Utc;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

use ethers_core::types::U256;
use rand::{rngs::StdRng, Rng, SeedableRng};
use strum::EnumCount;

use crate::game_core::board::{
    Board, BoardState, ClearValueDisplay, MoveAction, PlayerAction, SkillAction,
};
use crate::game_core::character::{
    AttackDecision, CharacterLogicData, CharacterV2, Command, EnemyScriptMap, EnemyTemplate,
};
use crate::game_core::config::{
    Bead, ClearPattern, DamageResult, DamageSource, DungeonGamer, Element, GameplayConfigManager,
    GetHitRecoveryType, BOARD_HEIGHT, BOARD_NUM_COLORS, BOARD_WIDTH, DEFAULT_ENEMY_SCRIPT_NAME,
    DEFAULT_ENEMY_TEMPLATE_NAME, ENEMY_ADDR, MAX_PARTY_MEMBER, MAX_ZONE_RECORD_SIZE, RATE_UNIT,
};
use crate::game_core::event_module::{update_event, GameEvent, GamerMove};
use crate::game_core::probability_mod::is_new_character_get;
use crate::game_core::room_manager::GameMode;
use crate::game_core::skill::{BuffInfo, SkillInfo};
use crate::game_core::{DazzleError, GameError, ServerError};

use super::board::WaitAction;
use super::reward::{CharacterReward, CurrencyReward, Reward, RewardType};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Gamer {
    pub id: String,
    pub is_ready: bool,
    pub is_quit_room: bool,
    pub character_uuid_list: Vec<Uuid>,
    pub stake: String,
}

impl Gamer {
    pub fn new(player_id: &str, character_uuid_list: &[Uuid], stake: &str) -> Self {
        Self {
            id: player_id.to_lowercase(),
            is_ready: true,
            is_quit_room: false,
            character_uuid_list: character_uuid_list.to_vec(),
            stake: stake.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameResult {
    pub score_record: ScoreRecord,
    pub game_over_result: GameOverResult,

    //#NOTE: Current used in Game mode that would directly give out character reward when winning, ex: PVP / Tutorial, maybe will be removed in future
    pub reward: Reward,

    //#NOTE: Fields below used in Game mode that will give out multiple reward choices when the game is over, ex: PVE / Rift
    pub reward_types: Vec<RewardType>,
    pub character_rewards: Vec<CharacterReward>,
    pub currency_rewards: Vec<CurrencyReward>,
}

impl GameResult {
    pub fn get_winner_id(&self) -> Option<&String> {
        self.game_over_result.get_winner_id()
    }

    pub fn get_dazzle_point(&self) -> u32 {
        self.score_record.get_dazzle_point()
    }

    pub fn is_winner(&self, player_id: &str) -> bool {
        self.get_winner_id()
            .map_or(false, |winner| *winner == player_id)
    }

    pub fn eval_elo_score(r_a: i32, r_b: i32) -> (u32, u32) {
        let exponent_a = (r_a - r_b) as f64 / 400.0;
        let exponent_b = -exponent_a;

        let expected_value_a = 1.0 / (1.0 + 10.0_f64.powf(exponent_a));
        let expected_value_b = 1.0 / (1.0 + 10.0_f64.powf(exponent_b));

        log::debug!(
            "   expected_value_a: {}, expected_value_b: {}",
            expected_value_a,
            expected_value_b
        );

        let new_r_a = r_a + (Self::eval_k_value() * (1.0 - expected_value_a)).round() as i32;
        let new_r_b = r_b + (Self::eval_k_value() * (0.0 - expected_value_b)).round() as i32;

        (new_r_a as u32, new_r_b as u32)
    }

    // TODO: Can be evaluate separately
    pub fn eval_k_value() -> f64 {
        32.0
    }
}

impl Default for GameResult {
    fn default() -> Self {
        GameResult {
            score_record: Default::default(),
            game_over_result: Default::default(),
            reward: Default::default(),
            reward_types: Default::default(),
            character_rewards: Default::default(),
            currency_rewards: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterSurvive {
    character_uuid: Uuid,
    is_dead: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreRecord {
    character_survive: Vec<CharacterSurvive>,
    rounds: u32,
    decay_rate: u32,
    energy_spent: u32,
    gems_cleared: Vec<u32>,
    highest_combo: usize,
    total_damage: i32,
    dazzle_point: u32,
}

impl ScoreRecord {
    pub fn get_dazzle_point(&self) -> u32 {
        self.dazzle_point
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameOverResult {
    pub winner: i8,
    pub winner_id: Option<String>,
    pub forfeit_game: bool,
    pub winner_reward: String,
    pub acquire_new_character: bool,
    pub nft_reward_dispatched: bool,
}

impl Default for GameOverResult {
    fn default() -> Self {
        GameOverResult {
            winner: -1,
            winner_id: Default::default(),
            forfeit_game: Default::default(),
            winner_reward: Default::default(),
            acquire_new_character: Default::default(),
            nft_reward_dispatched: Default::default(),
        }
    }
}

impl GameOverResult {
    pub fn get_winner_id(&self) -> Option<&String> {
        self.winner_id.as_ref()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Room {
    pub uuid: Uuid,
    pub private_code: String,
    pub game_mode: GameMode,
    pub opt_dungeon_details: Option<DungeonDetails>,
    pub gamers: Vec<Gamer>,
    pub start_with: usize,
    pub game: Game,
    pub game_over_result: Option<GameOverResult>,
    pub opt_reward_character_uuid: Option<Uuid>,
}

impl Serialize for Room {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut len = 6;
        if self.is_finished() {
            len = 7;
        }

        let mut room = serializer.serialize_struct("Room", len)?;
        room.serialize_field("uuid", &self.uuid)?;
        room.serialize_field("private_code", &self.private_code)?;
        room.serialize_field("game_mode", &self.game_mode)?;
        room.serialize_field("gamers", &self.gamers)?;
        room.serialize_field("start_with", &self.start_with)?;
        room.serialize_field("game", &self.game)?;

        if self.is_finished() {
            room.serialize_field("game_over_result", &self.game_over_result)?;
        }
        room.end()
    }
}

impl Room {
    pub fn new(
        private_code: Option<String>,
        game_mode: GameMode,
        opt_dungeon_details: Option<DungeonDetails>,
    ) -> Self {
        Room {
            uuid: Uuid::new_v4(),
            private_code: private_code.unwrap_or_else(|| String::new()),
            game_mode,
            opt_dungeon_details,
            gamers: vec![],
            start_with: 0,
            game: Default::default(),
            game_over_result: Default::default(),
            opt_reward_character_uuid: Default::default(),
        }
    }

    pub fn get_mover_idx(&self, player_id: &str) -> Result<usize, GameError> {
        self.gamers
            .iter()
            .position(|g| g.id == player_id)
            .ok_or_else(|| GameError::UserNotFound)
    }

    // TODO: Should refactor this function.
    // It has two responsibilities: 1. Set player into `room` 2. Create `game.states` and update gamers in first state.
    pub fn set_player(
        &mut self,
        player_id: &str,
        party_characters: &[CharacterV2],
        stake: &str,
        config: &GameplayConfigManager,
        seed: Option<u64>,
        _opt_dungeon_lv: Option<u32>, // Not being used currently
        opt_stage_lv: Option<u32>,
    ) {
        let character_uuid_list = party_characters
            .iter()
            .map(|c| *c.get_id())
            .collect::<Vec<Uuid>>();

        let gamer = Gamer::new(player_id, &character_uuid_list, stake);
        self.gamers.push(gamer);

        let gamer_state = GamerState::new(player_id, party_characters, config);

        if self.game.states.len() == 0 {
            let opt_dungeon_state = match self.game_mode {
                GameMode::DungeonRBS => Some(DungeonState::init(opt_stage_lv.unwrap())),
                _ => None,
            };

            // Init game states
            self.game = Game::new(
                BOARD_NUM_COLORS,
                BOARD_WIDTH,
                BOARD_HEIGHT,
                opt_dungeon_state,
                seed,
            );
        }
        self.game.states[0].gamer.push(gamer_state);
    }

    pub fn push_next_dungeon_enemy_state(
        &mut self,
        next_enemy_party: &[CharacterV2],
        config: &GameplayConfigManager,
    ) -> Result<(), DazzleError> {
        self.update_enemy_gamer(&next_enemy_party);

        let new_enemy_gamer = GamerState::new(ENEMY_ADDR, next_enemy_party, config);
        let last_state = self
            .game
            .states
            .last()
            .ok_or(GameError::CreateDungeonStageFailed)?
            .clone();

        let next_state = GameState::init_next_dungeon_stage(last_state, new_enemy_gamer);
        self.game.states.push(next_state);

        Ok(())
    }

    fn update_enemy_gamer(&mut self, party_characters: &[CharacterV2]) {
        let character_uuid_list: Vec<Uuid> = party_characters
            .iter()
            .map(|c| c.attribute.get_id().clone())
            .collect();
        self.gamers[DungeonGamer::Enemy as usize].character_uuid_list =
            character_uuid_list.to_owned()
    }

    /// If the game is not in dungeon mode, it will return `false`.
    pub fn is_dungeon_stage_clear(&self) -> bool {
        self.game.states.last().map_or(false, |game_state| {
            game_state
                .opt_dungeon_state
                .as_ref()
                .map_or(false, |dungeon_state| dungeon_state.is_stage_clear)
        })
    }

    /// If the game is not in dungeon mode, it will return `Ok(None)`.
    pub fn get_dungeon_stage_lv(&self) -> Result<Option<u32>, GameError> {
        self.game
            .states
            .last()
            .ok_or(GameError::NoGameState)
            .map(|last_state| {
                last_state
                    .opt_dungeon_state
                    .map(|dungeon_state| dungeon_state.stage_lv)
            })
    }

    pub fn get_dungeon_player_id(&self) -> Result<&str, GameError> {
        Ok(&self
            .gamers
            .iter()
            .find(|gamer| gamer.id != ENEMY_ADDR)
            .ok_or(GameError::UserNotFound)?
            .id)
    }

    pub fn get_participants_id(&self) -> Vec<String> {
        self.gamers.iter().map(|gamer| gamer.id.clone()).collect()
    }

    pub fn get_gamers_id(&self) -> Vec<String> {
        self.gamers
            .iter()
            .map(|gamer| gamer.id.clone())
            .collect::<Vec<String>>()
    }

    pub fn get_winner_id(&self) -> Option<String> {
        if !self.is_finished() {
            return None;
        }
        self.game_over_result.as_ref()?.get_winner_id().cloned()
    }

    pub fn check_mover(&self, player: &str) -> Result<(), GameError> {
        match self.gamers[self.game.current_active_player_idx].id == *player {
            true => Ok(()),
            false => {
                log::error!(
                    "\nCurrent mover id:{}\nValid mover id: {}",
                    player,
                    self.gamers[self.game.current_active_player_idx].id
                );
                Err(GameError::IllegalMove)
            }
        }
    }

    pub fn check_legal_move(&self, action: &MoveAction) -> Result<(), GameError> {
        if action.x >= BOARD_WIDTH || action.y >= BOARD_HEIGHT {
            log::error!("Try to move a bead out of boundary!");
            return Err(GameError::IllegalMove.into());
        }
        Ok(())
    }

    pub fn update_game(
        &mut self,
        mover: usize,
        action: &MoveAction,
        attacker_id: &Uuid,
        defender_id: &Uuid,
        config: &GameplayConfigManager,
    ) -> Result<(), GameError> {
        log::debug!("# Round: {}", (self.game.turn + 1) / 2);
        match self.game.states.last() {
            Some(state) => {
                let mut game_state_manager = GameResourceManager::init(
                    state,
                    self.game_mode,
                    false,
                    attacker_id,
                    Some(defender_id),
                    config,
                    self.game.rng.clone(),
                );

                let next_game_state =
                    game_state_manager.compose_next_state(self.game.turn, action, mover)?;

                if self.game_mode == GameMode::DungeonRBS {
                    let stage_lv = self
                        .get_dungeon_stage_lv()?
                        .ok_or(GameError::InvalidOperation)?;

                    let dungeon_details = self
                        .opt_dungeon_details
                        .clone()
                        .ok_or(GameError::DungeonNotFound)?;

                    self.set_reward_character_uuid()?;

                    if !dungeon_details.is_next_stage_exist(stage_lv + 1) {
                        if let Some(winner) = self.check_game_winner(&next_game_state) {
                            self.set_game_result(winner, &self.gamers[winner].id.clone(), false)?;
                        }
                    }
                } else {
                    if let Some(winner) = self.check_game_winner(&next_game_state) {
                        self.set_game_result(winner, &self.gamers[winner].id.clone(), false)?;
                    }
                }

                self.game.switch_player();
                self.game.states.push(next_game_state);
                self.game.total_states_count = self.game.states.len();

                log::warn!("   Compose state:[{}] complete", self.game.states.len() - 1);
                Ok(())
            }
            None => Err(GameError::NoGameState),
        }
    }

    pub fn activate_skill(
        &mut self,
        mover: usize,
        caster_id: Uuid,
        ally_target_id: Uuid,
        rival_target_id: Option<Uuid>,
        config: &GameplayConfigManager,
    ) -> Result<(), GameError> {
        match self.game.states.last() {
            Some(state) => {
                let mut game_state_manager = GameResourceManager::init(
                    state,
                    self.game_mode,
                    true,
                    &caster_id,
                    rival_target_id.as_ref(),
                    config,
                    self.game.rng.clone(),
                );

                let next_game_state = game_state_manager.compose_next_skill_state(
                    self.game.turn,
                    mover,
                    ally_target_id,
                )?;

                if self.game_mode == GameMode::DungeonRBS {
                    let stage_lv = self
                        .get_dungeon_stage_lv()?
                        .ok_or(GameError::InvalidOperation)?;

                    let dungeon_details = self
                        .opt_dungeon_details
                        .clone()
                        .ok_or(GameError::DungeonNotFound)?;

                    self.set_reward_character_uuid()?;

                    if !dungeon_details.is_next_stage_exist(stage_lv + 1) {
                        if let Some(winner) = self.check_game_winner(&next_game_state) {
                            self.set_game_result(winner, &self.gamers[winner].id.clone(), false)?;
                        }
                    }
                } else {
                    if let Some(winner) = self.check_game_winner(&next_game_state) {
                        self.set_game_result(winner, &self.gamers[winner].id.clone(), false)?;
                    }
                }

                self.game.states.push(next_game_state);
                self.game.total_states_count = self.game.states.len();
                log::warn!("   Compose state:[{}] complete", self.game.states.len() - 1);
                Ok(())
            }
            None => Err(GameError::NoGameState),
        }
    }

    pub fn update_enemy_turn(
        &mut self,
        mover: usize,
        config: &GameplayConfigManager,
        enemy_script_map: &EnemyScriptMap,
    ) -> Result<(), GameError> {
        let player = 1 - mover;
        let enemy = mover;

        log::debug!("# Enemy Turn: {}", self.game.turn);

        let alive_enemy_list = self.get_all_alive_character_ids(&self.gamers[enemy].id)?;
        log::debug!("Alive Enemies:\n{:#?}", alive_enemy_list);

        if alive_enemy_list.is_empty() {
            log::debug!("All enemies are dead");
            self.game.switch_player();
            return Ok(());
        }

        for attacker_id in &alive_enemy_list {
            match self.game.states.last() {
                Some(state) => {
                    // TODO: Need a mechanism to assign script name. Currently using a hard-coded script.
                    let command = enemy_script_map
                        .get_command(DEFAULT_ENEMY_SCRIPT_NAME, self.game.turn as usize)
                        .unwrap_or_else(|e| {
                            let default_command = Command::default();
                            log::warn!("{}", e.to_string());
                            log::debug!(
                                "    Using default command: {:?}",
                                default_command.command_type
                            );
                            default_command
                        });

                    let attacker_element = state.gamer[enemy]
                        .get_character_logic_data(attacker_id)?
                        .element;

                    if let Some(defender_id) = self.select_defender_target(
                        &state.gamer[player].characters,
                        attacker_element,
                        &command,
                    )? {
                        let mut game_state_manager = GameResourceManager::init(
                            state,
                            self.game_mode,
                            false,
                            attacker_id,
                            Some(defender_id),
                            config,
                            self.game.rng.clone(),
                        );

                        let next_game_state = game_state_manager.compose_next_npc_enemy_state(
                            self.game.turn,
                            command,
                            enemy,
                        )?;

                        if let Some(winner) = self.check_game_winner(&next_game_state) {
                            self.set_game_result(winner, &self.gamers[winner].id.clone(), false)?;
                        }

                        self.game.states.push(next_game_state);
                        self.game.total_states_count = self.game.states.len();
                        log::warn!("   Compose state:[{}] complete", self.game.states.len() - 1);
                    }
                }
                None => return Err(GameError::NoGameState),
            }
        }
        self.game.switch_player();
        Ok(())
    }

    fn check_game_winner(&self, state: &GameState) -> Option<usize> {
        // If player has no characters HP left, return the opponent index
        state
            .gamer
            .iter()
            .enumerate()
            .find(|(_, s)| s.characters.iter().fold(0, |acc, x| acc + x.current_hp) == 0)
            .map(|(index, _)| 1 - index)
    }

    pub fn set_game_forfeit(&mut self, forfeit_player_id: &str) -> Result<(), ServerError> {
        // Check game_over_result
        if self.game_over_result.is_some() {
            return Ok(());
        }

        for (i, gamer) in self.gamers.iter().enumerate() {
            if forfeit_player_id == gamer.id {
                self.set_game_result(1 - i, &self.gamers[1 - i].id.clone(), true)
                    .map_err(|_| ServerError::UserNotFound)?;
                return Ok(());
            }
        }

        Err(ServerError::UserNotFound)
    }

    pub fn is_finished(&self) -> bool {
        if let Some(game_over_result) = self.game_over_result.as_ref() {
            game_over_result.winner > -1
        } else {
            false
        }
    }

    pub fn is_all_players_quit(&self) -> bool {
        match self.game_mode {
            GameMode::PvP | GameMode::Cartesi | GameMode::Debug => {
                self.gamers.iter().all(|gamer| gamer.is_quit_room)
            }
            GameMode::PvE | GameMode::Tutorial | GameMode::DungeonRBS => {
                self.gamers.iter().any(|gamer| gamer.is_quit_room)
            }
        }
    }

    pub fn get_game_over_result(&self) -> Result<GameOverResult, GameError> {
        if !self.is_finished() || self.game_over_result.is_none() {
            return Err(GameError::InvalidOperation);
        }
        Ok(self.game_over_result.clone().unwrap())
    }

    pub fn get_game_reward(&self) -> Result<Reward, GameError> {
        let game_over_result = self
            .game_over_result
            .as_ref()
            .ok_or(GameError::InvalidOperation)?;

        Ok(Reward {
            winner_reward: game_over_result.winner_reward.clone(),
            acquire_new_character: game_over_result.acquire_new_character,
        })
    }

    pub fn set_game_result(
        &mut self,
        winner: usize,
        winner_id: &str,
        forfeit_game: bool,
    ) -> Result<(), GameError> {
        if self.gamers.iter().find(|c| c.id == winner_id).is_none() {
            return Err(GameError::UserNotFound);
        }

        // Calculate and setting stake reward
        let reward_stake = self.gamers.iter().fold(U256::zero(), |acc, x| {
            acc.saturating_add(U256::from_dec_str(&x.stake).unwrap())
        });

        self.game_over_result = Some(GameOverResult {
            winner: winner as i8,
            winner_id: Some(winner_id.to_owned()),
            forfeit_game,
            winner_reward: reward_stake.to_string(),
            acquire_new_character: self.is_mode_dispatch_nft() && is_new_character_get(winner_id),
            nft_reward_dispatched: false,
        });

        Ok(())
    }

    pub fn set_reward_character_uuid(&mut self) -> Result<(), GameError> {
        if self.opt_reward_character_uuid.is_some() {
            // Already set
            return Ok(());
        }

        let left_list = self.get_all_alive_character_ids(ENEMY_ADDR)?;
        let left_count = left_list.len();

        if left_count == 1 {
            let last_enemy = left_list.first().unwrap().clone();
            self.opt_reward_character_uuid = Some(last_enemy);
        }

        Ok(())
    }

    pub fn remove_reward_character_uuid(&mut self) {
        self.opt_reward_character_uuid = None;
    }

    pub fn cal_score_result(
        &self,
        player_id: &str,
        config: &GameplayConfigManager,
    ) -> Result<ScoreRecord, GameError> {
        let rounds = (self.game.turn as u32 + 1) / 2;
        let mut highest_combo = 0;
        let mut energy_spent = 0;
        let mut gems_cleared = vec![0; 5];
        let mut total_damage = 0;

        let mover_idx = self.get_mover_idx(player_id)?;

        // TODO: For greater efficiency, calculations should be performed during gameplay.
        // Using brute force loop to parse every state data for now.
        let states = &self.game.states;
        states.iter().for_each(|s| {
            if mover_idx == s.mover {
                // Update total damage
                total_damage += s
                    .damage_result
                    .iter()
                    .map(|d| d.defender_received_damage)
                    .filter(|&damage| damage > 0)
                    .sum::<i32>();

                s.board_states.iter().for_each(|b_state| {
                    let current_combo = match b_state {
                        BoardState::ClearState {
                            clear_mask: _,
                            combo_states,
                        } => {
                            combo_states.iter().for_each(|c_state| {
                                // Update gems_cleard
                                gems_cleared[c_state.color] += c_state.amount;
                            });

                            combo_states.len()
                        }
                        _ => 0,
                    };

                    // Update highest_combo
                    highest_combo = std::cmp::max(highest_combo, current_combo);
                });

                // Parse skill usage
                if let Some(skill_action) = &s.player_action.skill_action {
                    energy_spent += skill_action.skill_info.get_config_energy_per_cast();
                }
            }
        });

        let character_survive = self.game.get_survived_characters(player_id)?;
        let decay_rate = self.cal_decay_rate(rounds, &config);
        let raw_score = energy_spent
            + total_damage as u32 / 100
            + highest_combo as u32 * 10
            + gems_cleared
                .iter()
                .fold(0, |sum, clear_count| sum + clear_count)
                * 10;

        log::debug!(
            "raw score: {}, decay_rate: {}",
            raw_score,
            (100.0 * decay_rate).round() as u32
        );

        let dazzle_point = (raw_score as f64 * decay_rate).ceil() as u32;

        Ok(ScoreRecord {
            character_survive,
            rounds,
            decay_rate: (100.0 * decay_rate).round() as u32,
            energy_spent,
            gems_cleared,
            highest_combo,
            total_damage,
            dazzle_point,
        })
    }

    fn cal_decay_rate(&self, rounds: u32, config: &GameplayConfigManager) -> f64 {
        let (round_decay_threshold, round_cap) = config.get_rounds_decay_param();

        if rounds <= round_decay_threshold {
            return 1.0;
        } else if rounds >= round_cap {
            return 0.0;
        }

        // Linear decay
        let interval = round_cap - round_decay_threshold;
        1.0 - ((rounds - round_decay_threshold) as f64 / interval as f64)
    }

    fn get_all_alive_character_ids(&self, player_id: &str) -> Result<Vec<Uuid>, GameError> {
        Ok(self
            .game
            .get_gamer_state(player_id)?
            .get_all_alive_character_ids())
    }

    fn select_defender_target<'a>(
        &'a self,
        character_data_list: &'a [CharacterLogicData],
        attacker_element: Element,
        command: &Command,
    ) -> Result<Option<&Uuid>, GameError> {
        if !command.is_attack_action() {
            return Ok(None);
        }

        let candidate_list: Vec<&CharacterLogicData> = character_data_list
            .iter()
            .filter(|character| character.is_alive())
            .collect();

        log::debug!(
            "candidate_list:\n{:#?}",
            candidate_list.iter().map(|c| c.id).collect::<Vec<Uuid>>()
        );

        if candidate_list.is_empty() {
            log::debug!("No available target",);
            return Ok(None);
        }

        let defender_id = match command.attack_decision {
            AttackDecision::Random => {
                let random_pick = rand::thread_rng().gen_range(0..candidate_list.len());
                &candidate_list[random_pick].id
            }
            AttackDecision::LowestHp => candidate_list
                .iter()
                .min_by_key(|char_data| char_data.current_hp)
                .map(|char_data| &char_data.id)
                .unwrap(),
            AttackDecision::BenefitElement => {
                let advantage_target_element = attacker_element.get_advantage_element()?;
                // Collect all advantage target characters
                let mut filtered_id_list: Vec<&Uuid> = candidate_list
                    .iter()
                    .filter(|character| character.element == advantage_target_element)
                    .map(|character| &character.id)
                    .collect();

                if filtered_id_list.is_empty() {
                    // Sceondary priotity: exclude disadvantage target
                    let disadvantage_target_element =
                        attacker_element.get_disadvantage_element()?;

                    filtered_id_list = candidate_list
                        .iter()
                        .filter(|character| character.element != disadvantage_target_element)
                        .map(|character| &character.id)
                        .collect();
                }

                if filtered_id_list.is_empty() {
                    // No good target, random pick
                    let random_pick = rand::thread_rng().gen_range(0..candidate_list.len());
                    &candidate_list[random_pick].id
                } else {
                    let random_pick = rand::thread_rng().gen_range(0..filtered_id_list.len());
                    filtered_id_list[random_pick]
                }
            }
        };

        log::debug!(
            "Attack decision: {:?}, target: {}",
            command.attack_decision,
            defender_id
        );

        Ok(Some(defender_id))
    }

    pub fn is_mode_dispatch_nft(&self) -> bool {
        matches!(self.game_mode, GameMode::PvP | GameMode::DungeonRBS)
    }

    pub fn set_nft_reward_dispatched(&mut self) {
        if let Some(result) = self.game_over_result.as_mut() {
            result.nft_reward_dispatched = true;
        }
    }

    pub fn snapshot(&self) -> Room {
        let states = &self.game.states;
        let total_states_count = states.len();

        let games = states.last().map_or_else(Vec::new, |last_state| {
            states
                .iter()
                .filter(|s| s.turn == last_state.turn)
                .cloned()
                .collect()
        });

        //#NOTES: temp solution, we may need to review Unity's code to see if we really need to return a whole Room as response
        let snapshot_game = Game {
            current_active_player_idx: self.game.current_active_player_idx,
            turn: self.game.turn,
            total_states_count,
            states: games,
            //seed: 0i64,
            rng: StdRng::seed_from_u64(0),
        };

        let snapshot_room = Room {
            uuid: self.uuid,
            private_code: self.private_code.clone(),
            game_mode: self.game_mode,
            opt_dungeon_details: Default::default(),
            gamers: self.gamers.clone(),
            start_with: self.start_with,
            game: snapshot_game,
            game_over_result: self.game_over_result.clone(),
            opt_reward_character_uuid: None,
        };

        snapshot_room
    }

    #[cfg(feature = "debug_tool")]
    pub fn replace_import_board(
        &mut self,
        import_board_data: &[[u32; 14]; 5],
    ) -> Result<(), GameError> {
        match self.game.states.last() {
            Some(state) => {
                let mut current_state = state.clone();
                let next_game_state = current_state.next_replace_board_state(
                    self.game.turn,
                    self.game.current_active_player_idx,
                    import_board_data,
                )?;

                self.game.states.push(next_game_state);
                self.game.total_states_count = self.game.states.len();
                Ok(())
            }
            None => Err(GameError::NoGameState),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub current_active_player_idx: usize,
    pub turn: u8,
    pub total_states_count: usize,
    pub states: Vec<GameState>,

    #[serde(skip, default = "default_rng")]
    rng: StdRng,
}

fn default_rng() -> StdRng {
    StdRng::seed_from_u64(0)
}

impl Default for Game {
    fn default() -> Game {
        Game {
            total_states_count: 0,
            states: vec![],
            current_active_player_idx: 0,
            turn: 0,
            rng: StdRng::seed_from_u64(0),
        }
    }
}

impl Game {
    pub fn new(
        num_colors: u32,
        width: u32,
        height: u32,
        opt_dungeon_state: Option<DungeonState>,
        seed: Option<u64>,
    ) -> Self {
        let time = Utc::now();
        let rng_seed = seed.unwrap_or_else(|| time.timestamp() as u64);
        let mut rng = StdRng::seed_from_u64(rng_seed);

        let start_with = 0; // Should be random pick in the future

        let mut states = vec![];
        states.push(GameState::init(
            &mut rng,
            num_colors,
            width,
            height,
            opt_dungeon_state,
        ));

        Game {
            total_states_count: states.len(),
            states,
            turn: 1,
            current_active_player_idx: start_with,
            rng,
        }
    }

    pub fn get_gamer_state(&self, player: &str) -> Result<&GamerState, GameError> {
        self.states
            .last()
            .ok_or_else(|| GameError::NoGameState)?
            .gamer
            .iter()
            .find(|g_s| g_s.player == player)
            .ok_or_else(|| GameError::UserNotFound)
    }

    pub fn get_survived_characters(
        &self,
        player_id: &str,
    ) -> Result<Vec<CharacterSurvive>, GameError> {
        // Get the last state and find corresponding player's characters
        Ok(self
            .states
            .last()
            .ok_or_else(|| GameError::NoGameState)?
            .gamer
            .iter()
            .find(|g| g.player == player_id)
            .ok_or_else(|| GameError::UserNotFound)?
            .characters
            .iter()
            .map(|c| CharacterSurvive {
                character_uuid: c.id,
                is_dead: c.current_hp == 0,
            })
            .collect::<Vec<CharacterSurvive>>())
    }

    pub fn update_rng(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed as u64);
    }

    pub fn switch_player(&mut self) {
        self.current_active_player_idx = 1 - self.current_active_player_idx;
        self.turn += 1;
        log::warn!("switch to [{}]", self.current_active_player_idx);
    }
}

// Won't be serialize, just for controll complex `GameState` related resource during gameplay
#[derive(Debug, Clone)]
pub struct GameResourceManager<'a> {
    next_state: GameState,
    game_mode: GameMode,
    is_skill_state: bool,
    attacker_id: &'a Uuid,         // "attacker" in move, "caster" in skill
    defender_id: Option<&'a Uuid>, // "defender" in move, "target" in skill
    config: &'a GameplayConfigManager,
    rng: StdRng, // TODO: May replace to RandomHalder in the future
}

impl<'a> GameResourceManager<'a> {
    pub fn init(
        state: &GameState,
        game_mode: GameMode,
        is_skill_state: bool,
        attacker_id: &'a Uuid,
        defender_id: Option<&'a Uuid>,
        config: &'a GameplayConfigManager,
        rng: StdRng,
    ) -> Self {
        Self {
            next_state: state.clone(),
            game_mode,
            is_skill_state,
            attacker_id,
            defender_id,
            config,
            rng,
        }
    }

    pub fn compose_next_state(
        &mut self,
        current_turn: u8,
        action: &MoveAction,
        mover: usize,
    ) -> Result<GameState, GameError> {
        // Update mover's move buffer
        let mut next_gamer = self.next_state.gamer.clone();
        next_gamer[mover].update_move_buffer(&self.next_state.board, action);
        log::debug!(
            " ### Current move buffer:\n{:#?}",
            next_gamer[mover].move_buffer
        );

        // Do swap gem
        let mut board_states = self
            .next_state
            .board
            .simulate(action, &mut self.rng.clone())?;

        // Check event triggering
        let next_event = update_event(
            &self.next_state.game_event,
            &mut next_gamer[mover].move_buffer,
            current_turn,
            self.config.get_zone_expired_turn(),
        );

        let damage_result =
            self.eval_damage_result(&next_gamer, &mut board_states, &next_event, mover)?;

        // update character state
        let rival = 1 - mover;
        next_gamer[rival].consume_buff(&damage_result)?;
        next_gamer[rival].minus_character_hp(&damage_result, self.config);
        next_gamer[mover].remove_expired_buff_states(current_turn);
        next_gamer[rival].remove_expired_buff_states(current_turn);
        next_gamer[mover].update_character_cool_down(board_states.last(), self.config);

        let next_dungeon_state = self.new_dungeon_state(&next_gamer[rival])?;

        Ok(GameState {
            board: self.next_state.board.clone(),
            turn: current_turn,
            game_event: next_event,
            mover,
            player_action: PlayerAction {
                move_action: Some(*action),
                skill_action: None,
                wait_action: None,
            },
            attacker_id: Some(self.get_attacker_id().clone()),
            defender_id: Some(self.get_defender_id()?.clone()),
            board_states,
            damage_result,
            opt_dungeon_state: next_dungeon_state,
            gamer: next_gamer,
        })
    }

    pub fn compose_next_skill_state(
        &mut self,
        current_turn: u8,
        mover: usize,
        ally_target_id: Uuid,
    ) -> Result<GameState, GameError> {
        let mut next_gamer = self.next_state.gamer.clone();
        let rival = 1 - mover;
        let caster_id = self.get_attacker_id().clone();
        let rival_target_id = self
            .get_defender_id()
            .map_or_else(|_| None, |u| Some(u))
            .cloned();

        let caster_char = next_gamer[mover]
            .get_character_logic_data(&caster_id)?
            .clone();

        caster_char.alive()?;

        if !caster_char.is_skill_ready() {
            return Err(GameError::SkillNotReady);
        }

        let skill_info = caster_char.get_skill_info();

        let (next_baord, damage_result, board_states, targets_id) = self.perform_skill(
            &mut next_gamer,
            current_turn,
            mover,
            &skill_info,
            &caster_char,
            ally_target_id,
            rival_target_id.as_ref(),
            false,
        )?;

        // SkillInfo::Damage will ignore any buff, thus the buff effect will not consumed.
        if caster_char.get_skill_info() != SkillInfo::Damage {
            next_gamer[rival].consume_buff(&damage_result)?;
        }

        next_gamer[rival].remove_expired_buff_states(current_turn);
        next_gamer[mover].consume_skill_cool_down(&caster_id)?;
        next_gamer[rival].minus_character_hp(&damage_result, self.config);

        let next_dungeon_state = self.new_dungeon_state(&next_gamer[rival])?;

        self.next_state.board_states = board_states.clone();
        Ok(GameState {
            board: next_baord,
            turn: current_turn,
            game_event: self.next_state.game_event.clone(),
            mover,
            player_action: PlayerAction {
                move_action: None,
                skill_action: Some(SkillAction {
                    skill_info: caster_char.get_skill_info(),
                    caster_id,
                    targets_id,
                }),
                wait_action: None,
            },
            attacker_id: Some(caster_id),
            defender_id: rival_target_id,
            board_states,
            damage_result,
            opt_dungeon_state: next_dungeon_state,
            gamer: next_gamer,
        })
    }

    pub fn compose_next_npc_enemy_state(
        &mut self,
        current_turn: u8,
        command: Command,
        enemy: usize,
    ) -> Result<GameState, GameError> {
        let player = 1 - enemy;
        let mut next_gamer = self.next_state.gamer.clone();

        let attacker_char = next_gamer[enemy]
            .get_character_logic_data(self.get_attacker_id())?
            .clone();
        let defender_char = next_gamer[player]
            .get_character_logic_data(self.get_defender_id()?)?
            .clone();

        let skill_info = command.skill_info.unwrap_or(SkillInfo::NpcAttack);

        let (next_borad, damage_result, board_states, targets_id) = self.perform_skill(
            &mut next_gamer,
            current_turn,
            enemy,
            &skill_info,
            &attacker_char,
            attacker_char.id,
            Some(&defender_char.id),
            true,
        )?;

        // SkillInfo::Damage will ignore any buff, thus the buff effect will not consumed.
        if skill_info != SkillInfo::Damage {
            next_gamer[player].consume_buff(&damage_result)?;
        }

        next_gamer[player].remove_expired_buff_states(current_turn);
        next_gamer[player].minus_character_hp(&damage_result, self.config);

        Ok(GameState {
            board: next_borad,
            turn: current_turn,
            game_event: self.next_state.game_event.clone(),
            mover: enemy,
            player_action: PlayerAction {
                move_action: None,
                skill_action: Some(SkillAction {
                    skill_info,
                    caster_id: attacker_char.id,
                    targets_id,
                }),
                wait_action: None,
            },
            attacker_id: Some(attacker_char.id),
            defender_id: Some(defender_char.id),
            board_states,
            damage_result,
            opt_dungeon_state: self.next_state.opt_dungeon_state,
            gamer: next_gamer,
        })
    }

    pub fn new_dungeon_state(
        &self,
        next_enemy_gamer: &GamerState,
    ) -> Result<Option<DungeonState>, GameError> {
        if !self.is_dungeon_and_stage_clear(next_enemy_gamer) {
            return Ok(self.next_state.opt_dungeon_state);
        }

        let stage_lv = self.get_current_dungeon_stage_lv()?;
        Ok(Some(DungeonState::clear_stage(stage_lv)))
    }

    pub fn is_dungeon_and_stage_clear(&self, next_enemy_gamer: &GamerState) -> bool {
        match self.game_mode {
            GameMode::DungeonRBS => next_enemy_gamer
                .characters
                .iter()
                .all(|enemy_char| enemy_char.current_hp == 0),
            _ => false,
        }
    }

    fn perform_skill(
        &self,
        next_gamer: &mut [GamerState],
        current_turn: u8,
        mover: usize,
        skill_info: &SkillInfo,
        caster_char: &CharacterLogicData,
        ally_target_id: Uuid,
        rival_target_id: Option<&Uuid>,
        is_npc_action: bool,
    ) -> Result<(Board, Vec<DamageResult>, Vec<BoardState>, Option<Vec<Uuid>>), GameError> {
        let rival = 1 - mover;
        let ally_id_list = next_gamer[mover]
            .characters
            .iter()
            .map(|c| c.id)
            .collect::<Vec<Uuid>>();

        let mut next_board = self.next_state.board.clone();

        let mut damage_result = vec![];
        let mut board_states = vec![];
        let targets_id = match skill_info {
            SkillInfo::None => {
                return Err(GameError::InvalidInput("should have skill name".to_owned()));
            }
            SkillInfo::ReplaceTestBoard => {
                return Err(GameError::InvalidInput(
                    "feature for testing, should not go here".to_owned(),
                ));
            }
            SkillInfo::NpcAttack => match rival_target_id {
                Some(defender_id) => {
                    let defender_char = next_gamer[rival].get_character_logic_data(&defender_id)?;
                    defender_char.alive()?;

                    let attacker_produced_damage =
                        self.eval_npc_normal_attack(&caster_char, &defender_char);

                    let dr = self.eval_attack_result(
                        caster_char,
                        defender_char,
                        attacker_produced_damage,
                        true,
                    )?;

                    damage_result.push(dr);
                    Some(vec![defender_id.clone()])
                }
                None => {
                    return Err(GameError::InvalidInput(
                        "should have target character".to_owned(),
                    ));
                }
            },
            SkillInfo::Damage => match rival_target_id {
                Some(defender_id) => {
                    let defender_char = next_gamer[rival].get_character_logic_data(&defender_id)?;
                    defender_char.alive()?;

                    let attacker_produced_damage =
                        SkillInfo::Damage.get_config_value() * caster_char.atk / RATE_UNIT;

                    // Damage skill currently not apply def or shield related logic
                    damage_result.push(DamageResult {
                        damage_source: DamageSource::SkillDamage,
                        attacker: caster_char.id,
                        defender: defender_id.clone(),
                        attacker_produced_damage,
                        defender_received_damage: attacker_produced_damage as i32,
                        shield_blocking: Default::default(),
                    });

                    Some(vec![defender_id.clone()])
                }
                None => {
                    return Err(GameError::InvalidInput(
                        "should have target character".to_owned(),
                    ));
                }
            },
            SkillInfo::Recovery => {
                next_gamer[mover]
                    .characters
                    .iter_mut()
                    .for_each(|ally_char| {
                        let recovery_val =
                            caster_char.atk * SkillInfo::Recovery.get_config_value() / RATE_UNIT;

                        if ally_char.recovery_hp(recovery_val) {
                            // Using negative damage to represent heal
                            damage_result.push(DamageResult {
                                damage_source: DamageSource::SkillRecovery,
                                attacker: caster_char.id,
                                defender: ally_char.id,
                                attacker_produced_damage: Default::default(),
                                defender_received_damage: recovery_val as i32 * -1,
                                shield_blocking: Default::default(),
                            });
                        }
                    });

                Some(ally_id_list.clone())
            }
            SkillInfo::DefenseAmplify => {
                next_gamer[mover]
                    .characters
                    .iter_mut()
                    .for_each(|c| c.add_buff_states(BuffInfo::DefenseAmplify, current_turn));

                Some(ally_id_list)
            }
            SkillInfo::TurnTiles => {
                let available_elements = next_board.remaining_colors_on_board();

                let from_elem = if is_npc_action
                    && !available_elements
                        .contains(&caster_char.element.get_disadvantage_element()?)
                {
                    // If skill is triggered by NPC and there is no available target, temporary using a random value to handle it
                    let mut picked_elem;
                    loop {
                        picked_elem = Element::from(
                            rand::thread_rng()
                                .gen_range(Element::Fire as u32..=Element::Shadow as u32),
                        );

                        if available_elements.contains(&picked_elem)
                            && picked_elem != caster_char.element
                        {
                            // Avoiding to pick invalid element or same element as `to_elem`
                            break;
                        }
                    }
                    picked_elem
                } else {
                    caster_char.element.get_disadvantage_element()?
                };

                board_states = next_board.turn_tiles(from_elem, caster_char.element)?;
                None
            }
            SkillInfo::AttackAmplify => {
                next_gamer[mover]
                    .characters
                    .iter_mut()
                    .for_each(|c| c.add_buff_states(BuffInfo::AttackAmplify, current_turn));

                Some(ally_id_list)
            }
            SkillInfo::ShieldNullify => {
                let ally_target_char = next_gamer[mover]
                    .characters
                    .iter_mut()
                    .find(|c| c.id == ally_target_id)
                    .ok_or_else(|| {
                        GameError::InvalidInput("should have target character".to_owned())
                    })?;

                ally_target_char.alive()?;

                ally_target_char.add_buff_states(BuffInfo::ShieldNullify, current_turn);

                Some(vec![ally_target_id])
            }
            SkillInfo::ShieldAbsorb => {
                let ally_target_char = next_gamer[mover]
                    .characters
                    .iter_mut()
                    .find(|c| c.id == ally_target_id)
                    .ok_or_else(|| {
                        GameError::InvalidInput("should have target character".to_owned())
                    })?;

                ally_target_char.alive()?;

                ally_target_char.add_buff_states(BuffInfo::ShieldAbsorb, current_turn);

                Some(vec![ally_target_id])
            }
            SkillInfo::ElementalExplosion => {
                let target_element = if is_npc_action {
                    // Temporary using a random value
                    Element::from(
                        rand::thread_rng().gen_range(Element::Fire as u32..=Element::Shadow as u32),
                    )
                } else {
                    caster_char
                        .get_skill_target_elem()
                        .ok_or(GameError::SkillParamError)?
                };

                board_states =
                    next_board.element_explosion(target_element, &mut self.rng.clone())?;

                damage_result.extend(self.eval_damage_result(
                    &next_gamer,
                    &mut board_states,
                    &self.next_state.game_event,
                    mover,
                )?);

                None
            }
            SkillInfo::LineEliminate => {
                let (line_num, clear_pattern) = if is_npc_action {
                    // Temporary using random values
                    let clear_pattern = ClearPattern::from(rand::thread_rng().gen_range(1..=2));
                    let max_value = match clear_pattern {
                        ClearPattern::Horizontal => BOARD_HEIGHT,
                        ClearPattern::Vertical => BOARD_WIDTH,
                        _ => unreachable!(),
                    };
                    let line_num = rand::thread_rng().gen_range(0..max_value);
                    (line_num, clear_pattern)
                } else {
                    let line_num = caster_char.get_skill_param_value();
                    let clear_pattern = caster_char
                        .get_skill_clear_pattern()
                        .ok_or(GameError::SkillParamError)?;
                    (line_num, clear_pattern)
                };

                board_states =
                    next_board.line_eleminate(clear_pattern, line_num, &mut self.rng.clone())?;

                damage_result.extend(self.eval_damage_result(
                    &next_gamer,
                    &mut board_states,
                    &self.next_state.game_event,
                    mover,
                )?);

                None
            }
        };
        Ok((next_board, damage_result, board_states, targets_id))
    }

    fn get_current_dungeon_stage_lv(&self) -> Result<u32, GameError> {
        self.next_state
            .opt_dungeon_state
            .as_ref()
            .ok_or(GameError::FetchDungeonStageFailed)
            .map(|dungeon_state| dungeon_state.stage_lv)
    }

    fn get_attacker_id(&self) -> &Uuid {
        self.attacker_id
    }

    fn get_defender_id(&self) -> Result<&Uuid, GameError> {
        self.defender_id.ok_or_else(|| GameError::CharacterNotFound)
    }

    fn eval_damage_result(
        &self,
        next_gamer: &[GamerState],
        result: &mut [BoardState],
        event: &Option<GameEvent>,
        mover: usize,
    ) -> Result<Vec<DamageResult>, GameError> {
        let rival = 1 - mover;
        // The first element in vec is the main attacker.
        let attacker_char_list =
            next_gamer[mover].get_attacker_character_logic_data_list(self.get_attacker_id())?;
        let defender_char = next_gamer[rival].get_character_logic_data(self.get_defender_id()?)?;

        attacker_char_list.iter().try_for_each(|c| c.alive())?;

        let attackers_produced_damage =
            self.eval_clear_state_damage(event, result, &attacker_char_list, &defender_char);

        let mut damage_result = vec![];
        for (idx, (attacker_char, damage)) in attackers_produced_damage.iter().enumerate() {
            damage_result.push(self.eval_attack_result(
                attacker_char,
                &defender_char,
                *damage,
                idx == 0,
            )?);
        }

        Ok(damage_result)
    }

    /// Return raw damage of each attacker characters produced in "ClearState".
    ///
    /// Also evaluate the intermediate damage valu to be displayed in Unity and set it into `combo_state`
    fn eval_clear_state_damage(
        &self,
        event: &'a Option<GameEvent>,
        result: &'a mut [BoardState],
        attacker_char_list: &'a [&CharacterLogicData],
        defender_char: &'a CharacterLogicData,
    ) -> Vec<(&'a CharacterLogicData, u32)> {
        let attacker_base_damage = attacker_char_list
            .iter()
            .enumerate()
            .map(|(i, attacker_char)| {
                let mut base_damage = self.eval_damage_base(attacker_char, defender_char);
                if i > 0 {
                    base_damage *= attacker_char.assist_nerf_modifier as f64 / RATE_UNIT as f64;
                }
                (*attacker_char, base_damage)
            })
            .collect::<Vec<(&CharacterLogicData, f64)>>();

        let gem_amount = self.set_intermediate_display_value(result, event, &attacker_base_damage);
        log::debug!(
            " ### gem_amount: ({}) color: {:?}",
            gem_amount.iter().sum::<u32>(),
            gem_amount
        );

        let attackers_final_damage = attacker_base_damage
            .iter()
            .map(|(attacker, base_damage)| {
                (
                    *attacker,
                    self.total_gems_amount_damage(event, *base_damage, &gem_amount)
                        .round() as u32,
                )
            })
            .collect::<Vec<(&CharacterLogicData, u32)>>();

        attackers_final_damage
    }

    fn eval_damage_base(
        &self,
        attacker: &CharacterLogicData,
        defender: &CharacterLogicData,
    ) -> f64 {
        let atk = attacker.get_total_atk();
        let def = defender.get_total_def();

        let damage = self.base_damage_formula(atk, def);
        log::debug!("    atk: {}, def: {}, damage: {:.2}\n", atk, def, damage);

        damage
    }

    fn base_damage_formula(&self, atk: u32, def: u32) -> f64 {
        let coef = self.config.get_damage_formula_coef();

        let damage = if atk >= def {
            coef.a * (atk.pow(coef.exp) as f64 / (atk as f64 + def as f64 * coef.b) as f64)
        } else {
            coef.b * (atk.pow(coef.exp) as f64 / (atk as f64 + def as f64 * coef.d) as f64)
        };

        damage
    }

    fn eval_npc_normal_attack(
        &self,
        attacker_char: &CharacterLogicData,
        defender_char: &CharacterLogicData,
    ) -> u32 {
        // Gems amount and color are not matter currently.
        let gem_amount: &[u32] = &[3, 0, 0, 0, 0];
        let base_damage = self.eval_damage_base(attacker_char, defender_char);

        self.total_gems_amount_damage(&None, base_damage, gem_amount)
            .round() as u32
    }

    fn eval_attack_result(
        &self,
        attacker: &CharacterLogicData,
        defender: &CharacterLogicData,
        attacker_produced_damage: u32,
        is_main_attacker: bool,
    ) -> Result<DamageResult, GameError> {
        let defender_received_damage = self
            .config
            .apply_element_modifier(
                attacker_produced_damage,
                &attacker.element,
                &defender.element,
            )
            .map_err(|_| GameError::CharacterElementError)?;

        log::debug!(
            "    Defender received damage raw = {}",
            defender_received_damage
        );

        let (defender_finalized_damage, has_shield) =
            defender.apply_shield_buff(defender_received_damage);
        log::debug!(
            "            ### Result: attacker_produce: {}, defender_received: {}",
            attacker_produced_damage,
            defender_finalized_damage
        );

        Ok(DamageResult {
            damage_source: if is_main_attacker {
                DamageSource::MainAttacker
            } else {
                DamageSource::AssistAttacker
            },
            attacker: attacker.id,
            defender: defender.id,
            attacker_produced_damage,
            defender_received_damage: defender_finalized_damage,
            shield_blocking: has_shield,
        })
    }

    // Evaluate and update combo_state value to be displayed in Unity.
    // The reason to set display value here because we can't get character attribute at borad clear stage.
    // So compose the `combo_state` information first at borad clear stage, then set the actual damage value here.
    fn set_intermediate_display_value(
        &self,
        result: &mut [BoardState],
        event: &Option<GameEvent>,
        attacker_base_damage: &[(&CharacterLogicData, f64)],
    ) -> [u32; Bead::COUNT] {
        let mut gem_amount = [0; Bead::COUNT];
        result.iter_mut().for_each(|state| {
            if let BoardState::ClearState {
                clear_mask: _,
                combo_states,
            } = state
            {
                combo_states.iter_mut().for_each(|c_state| {
                    gem_amount[c_state.color] += c_state.amount;
                    for (attacker_char, base_damage) in attacker_base_damage {
                        c_state.character_val_display.push(self.eval_display_damage(
                            attacker_char,
                            &gem_amount,
                            self.total_gems_amount_damage(event, *base_damage, &gem_amount),
                        ));
                    }
                });
            };
        });

        gem_amount
    }

    fn total_gems_amount_damage(
        &self,
        event: &Option<GameEvent>,
        base_damage: f64,
        gem_amount: &[u32],
    ) -> f64 {
        let raw_damage = base_damage * self.amount_modifier(gem_amount.iter().sum::<u32>());
        let extra_damage = if let Some(zone) = event {
            base_damage * self.config.get_zone_buff_rate() as f64 / RATE_UNIT as f64
                * self.amount_modifier(gem_amount[zone.info.target_elem as usize])
        } else {
            0.0
        };

        // log::debug!(
        //     "    raw_damage: {:.2}, extra damage: {:.2}, total: {:.2}",
        //     raw_damage,
        //     extra_damage,
        //     raw_damage + extra_damage
        // );

        raw_damage + extra_damage
    }

    fn eval_display_damage(
        &self,
        attacker_char: &CharacterLogicData,
        gem_amount: &[u32],
        damage: f64,
    ) -> ClearValueDisplay {
        let cd_added = self.eval_each_clear_state_cd(attacker_char, gem_amount);
        ClearValueDisplay {
            id: attacker_char.id,
            damage: damage.round() as u32,
            cd_added,
            cd_charged: attacker_char.get_current_cool_down() + cd_added,
        }
    }

    fn amount_modifier(&self, gems: u32) -> f64 {
        let decrease_rate = self.config.get_damage_formula_coef().decrease_rate;
        (1.0 - decrease_rate.powf(gems as f64)) / (1.0 - decrease_rate)
    }

    fn eval_each_clear_state_cd(
        &self,
        attacker_char: &CharacterLogicData,
        gem_amount: &[u32],
    ) -> u32 {
        if !self.is_skill_state {
            attacker_char.eval_skill_charge_by_clear(&gem_amount, self.config)
        } else {
            0
        }
    }
}

// Will be serialized for use in Unity
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameState {
    pub board: Board,
    pub turn: u8,
    pub game_event: Option<GameEvent>,
    pub mover: usize,
    pub player_action: PlayerAction,
    pub attacker_id: Option<Uuid>, // None is for game initialize
    pub defender_id: Option<Uuid>, // Some skill has no target.
    pub board_states: Vec<BoardState>,
    pub damage_result: Vec<DamageResult>,
    pub opt_dungeon_state: Option<DungeonState>, // `None` if the game is not `GameMode::Dungeon`
    pub gamer: Vec<GamerState>,
}

impl GameState {
    pub fn init(
        rng: &mut StdRng,
        num_colors: u32,
        width: u32,
        height: u32,
        opt_dungeon_state: Option<DungeonState>,
    ) -> Self {
        GameState {
            board: Board::new(rng, num_colors, width, height),
            turn: Default::default(),
            game_event: Default::default(),
            mover: Default::default(),
            player_action: Default::default(),
            attacker_id: Default::default(),
            defender_id: Default::default(),
            board_states: vec![],
            damage_result: vec![],
            opt_dungeon_state,
            gamer: Default::default(),
        }
    }

    pub fn init_next_dungeon_stage(last_state: Self, new_enemy_gamer: GamerState) -> Self {
        let opt_next_dungeon_state = last_state
            .opt_dungeon_state
            .map(|dungeon_state| DungeonState::increment_stage_lv(dungeon_state.stage_lv));

        Self {
            board: last_state.board,
            turn: last_state.turn,
            game_event: last_state.game_event,
            mover: last_state.mover,
            player_action: PlayerAction {
                move_action: None,
                skill_action: None,
                wait_action: Some(WaitAction {}),
            },
            attacker_id: last_state.attacker_id,
            defender_id: last_state.defender_id,
            board_states: Default::default(),
            damage_result: Default::default(),
            opt_dungeon_state: opt_next_dungeon_state,
            gamer: vec![
                last_state.gamer[DungeonGamer::Player as usize].clone(),
                new_enemy_gamer,
            ],
        }
    }

    #[cfg(feature = "debug_tool")]
    pub fn next_replace_board_state(
        &mut self,
        current_turn: u8,
        mover: usize,
        import_board_data: &[[u32; 14]; 5],
    ) -> Result<GameState, GameError> {
        self.board_states = self.board.replace_board(import_board_data);

        // Mock as a Turntiles skill
        let skill_action = SkillAction {
            skill_info: SkillInfo::ReplaceTestBoard,
            caster_id: Default::default(),
            targets_id: None,
        };

        Ok(GameState {
            board: self.board.clone(),
            turn: current_turn,
            game_event: self.game_event.clone(),
            mover,
            player_action: PlayerAction {
                move_action: None,
                skill_action: Some(skill_action),
                wait_action: None,
            },
            attacker_id: Default::default(),
            defender_id: None,
            board_states: self.board_states.clone(),
            damage_result: vec![],
            opt_dungeon_state: Default::default(),
            gamer: self.gamer.clone(),
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GamerState {
    pub player: String,
    pub characters: Vec<CharacterLogicData>,
    move_buffer: Vec<GamerMove>,
}

impl GamerState {
    pub fn new(
        player: &str,
        characters: &[CharacterV2],
        config: &GameplayConfigManager,
    ) -> GamerState {
        GamerState {
            player: player.to_owned(),
            move_buffer: vec![],
            characters: characters.iter().map(|c| c.extract_data(config)).collect(),
        }
    }

    pub fn get_all_alive_character_ids(&self) -> Vec<Uuid> {
        self.characters
            .iter()
            .filter_map(|c| c.is_alive().then(|| c.id))
            .collect()
    }

    pub fn get_first_alive_character_id(&self) -> Result<&Uuid, GameError> {
        Ok(&self
            .characters
            .iter()
            .find(|c| c.is_alive())
            .ok_or_else(|| GameError::CharacterNotFound)?
            .id)
    }

    fn minus_character_hp(
        &mut self,
        damage_result: &[DamageResult],
        config: &GameplayConfigManager,
    ) {
        let mut dbg_total_damage = 0;
        for dr in damage_result {
            if let Some(character) = self.characters.iter_mut().find(|c| c.id == dr.defender) {
                dbg_total_damage += dr.defender_received_damage;

                character.update_hp(character.current_hp as i32 - dr.defender_received_damage);

                // Update skill charge while character get hit
                let charge_info = config.get_charge_info();
                let energy_recovery_by_damage = match charge_info.recover_by_get_hit {
                    GetHitRecoveryType::None => 0,
                    GetHitRecoveryType::Fixed => charge_info.fixed_recovery_amount_by_hit,
                    GetHitRecoveryType::DamageRate => {
                        // Temp rule: Recovery proportionally by damage
                        // max_cd * (damage/max_hp) * config_rate, then get ceiling value
                        (character.get_max_skill_charge() as f64
                            * (dr.defender_received_damage as f64 / character.max_hp as f64)
                            * charge_info.recovery_by_damage_rate)
                            .ceil() as u32
                    }
                };

                log::debug!(
                    "      # Recover energy by damage: {}",
                    energy_recovery_by_damage
                );

                character.add_extra_cool_down(energy_recovery_by_damage);
            }
        }
        log::debug!("   # Total damage: {}", dbg_total_damage);
    }

    /// SPEC description:
    ///
    /// If buffer is empty, directly push new move.
    ///
    /// If zone has triggered (record buffer is full), clear buffer and push new move.
    ///
    /// If same color at last position, push new move, otherwise empty the buffer.
    fn update_move_buffer(&mut self, board: &Board, _move: &MoveAction) {
        let gamer_move = board.get_move_info(_move);
        let top = self.move_buffer.last();
        if top.is_none() {
            log::debug!("### Push [{:?}]", gamer_move.elem);
            self.move_buffer.push(gamer_move);
            return;
        }

        // In current SPEC, push if same color at last, or empty the buffer
        if let Some(top_move) = top {
            if top_move.elem == gamer_move.elem && self.move_buffer.len() < MAX_ZONE_RECORD_SIZE {
                log::debug!("### Push [{:?}]", gamer_move.elem);
                self.move_buffer.push(gamer_move);
            } else {
                log::debug!("$$$ Top [{:?}], Clear buffer", top_move.elem);
                let buffer_len = self.move_buffer.len();
                self.move_buffer.clear();
                if buffer_len == MAX_ZONE_RECORD_SIZE {
                    self.move_buffer.push(gamer_move);
                }
            }
        }
    }

    fn consume_buff(&mut self, damage_result: &[DamageResult]) -> Result<(), GameError> {
        // ### TODO:
        // Currently only one damage target(defender) in one request move, find first denfender's buff to consume is enough
        if let Some(dr) = damage_result.iter().find(|r| r.shield_blocking == true) {
            let defender_char = self
                .characters
                .iter_mut()
                .find(|c| c.id == dr.defender)
                .ok_or(GameError::CharacterNotFound)?;

            defender_char.consume_buff()
        };
        Ok(())
    }

    fn update_character_cool_down(
        &mut self,
        latest_state: Option<&BoardState>,
        config: &GameplayConfigManager,
    ) {
        if let Some(state) = latest_state {
            let remove_beads = match state {
                BoardState::FillState { board } | BoardState::RerollState { board } => {
                    &board.board_data.remove_bead
                }
                _ => return,
            };

            self.characters
                .iter_mut()
                .for_each(|c| c.update_cool_down(remove_beads, config));
        };
    }

    fn remove_expired_buff_states(&mut self, current_turn: u8) {
        self.characters.iter_mut().for_each(|c| {
            let buff_cnt = c.buff_states.len();
            c.remove_expired_buff_states(current_turn);

            if buff_cnt != c.buff_states.len() {
                log::debug!(
                    "   ### Update buff_states - char[{}]: {:?}",
                    c.id,
                    c.buff_states
                )
            }
        })
    }

    fn get_attacker_character_logic_data_list(
        &self,
        main_attacker_id: &Uuid,
    ) -> Result<Vec<&CharacterLogicData>, GameError> {
        let mut attacker_char_list = vec![];
        attacker_char_list.push(self.get_character_logic_data(main_attacker_id)?);
        attacker_char_list.extend(self.get_assist_attackers(main_attacker_id));

        Ok(attacker_char_list)
    }

    pub fn get_character_logic_data(
        &self,
        character_uuid: &Uuid,
    ) -> Result<&CharacterLogicData, GameError> {
        match self.characters.iter().find(|&c| c.id == *character_uuid) {
            Some(c) => Ok(c),
            None => Err(GameError::CharacterNotFound),
        }
    }

    fn get_assist_attackers(&self, main_attacker_id: &Uuid) -> Vec<&CharacterLogicData> {
        let mut assist_attackers = vec![];
        self.characters.iter().for_each(|c| {
            if c.is_alive() && c.id != *main_attacker_id {
                assist_attackers.push(c);
            }
        });

        assist_attackers
    }

    fn consume_skill_cool_down(&mut self, char_id: &Uuid) -> Result<(), GameError> {
        // TODO: Can be optimize to hashmap query
        match self.characters.iter_mut().find(|c| c.id == *char_id) {
            Some(c) => Ok(c.consume_cool_down()),
            None => Err(GameError::CharacterNotFound),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize)]
pub struct DungeonState {
    stage_lv: u32,
    is_stage_clear: bool,
}

impl DungeonState {
    pub fn init(stage_lv: u32) -> Self {
        Self {
            stage_lv,
            is_stage_clear: false,
        }
    }

    pub fn clear_stage(stage_lv: u32) -> Self {
        Self {
            stage_lv,
            is_stage_clear: true,
        }
    }

    pub fn increment_stage_lv(current_stage_lv: u32) -> Self {
        Self::init(current_stage_lv + 1)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DungeonDetails {
    pub dungeon_name: String,
    pub comment: String,
    pub stage_info_list: Vec<DungeonStageInfo>,
}

impl DungeonDetails {
    pub fn is_valid_param(&self) -> bool {
        self.stage_info_list
            .iter()
            .all(|stage_info| stage_info.is_valid_param())
    }

    pub fn is_next_stage_exist(&self, next_stage_lv: u32) -> bool {
        (next_stage_lv as usize) < self.stage_info_list.len()
    }

    pub fn get_stage_enemy_templ_list(&self, stage: u32) -> Vec<String> {
        self.stage_info_list
            .get(stage as usize)
            .map(|info| info.enemy_templ_name_list.clone())
            .unwrap_or_else(|| {
                log::debug!("No dungeon stage info, create default info");
                vec![DEFAULT_ENEMY_TEMPLATE_NAME.to_owned(); 3]
            })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DungeonStageInfo {
    enemy_templ_name_list: Vec<String>,
}

impl DungeonStageInfo {
    pub fn new(enemy_templ_list: &[EnemyTemplate]) -> Self {
        Self {
            enemy_templ_name_list: enemy_templ_list
                .iter()
                .map(|t| t.enemy_template_name.clone())
                .collect(),
        }
    }

    pub fn is_valid_param(&self) -> bool {
        self.enemy_templ_name_list.len() <= MAX_PARTY_MEMBER
    }
}
