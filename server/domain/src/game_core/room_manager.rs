use crate::game_core::board::MoveAction;
use crate::game_core::character::{CharacterV2, EnemyScriptMap};
#[cfg(feature = "debug_tool")]
use crate::game_core::config::TEST_BOARD_PATH;
use crate::game_core::config::{
    DungeonGamer, GameplayConfigManager, ENEMY_ADDR, PRIVATE_CODE_LENGTH, STAKE,
};
use crate::game_core::game::{DungeonDetails, GameResult, Gamer, Room};
use crate::game_core::skill::SkillInfo;
use crate::game_core::{DazzleError, ServerError};

use atb::prelude::*;
use atb_types::prelude::uuid::Uuid;
use rand::distributions::{Distribution, Uniform};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::EnumString;

use super::reward::RewardCache;
use super::users::UserProfile;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum GameMode {
    Tutorial,
    PvE,
    DungeonRBS,
    PvP,
    Cartesi,
    Debug,
}

impl GameMode {
    pub fn from_string(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    pub fn get_win_count_field(&self) -> String {
        match self {
            Self::PvE => String::from("pve_win_count"),
            Self::PvP => String::from("pvp_win_count"),
            Self::Cartesi => String::from("cartesi_win_count"),
            _ => unreachable!(),
        }
    }

    pub fn get_highest_score_field(&self) -> String {
        match self {
            Self::PvE => String::from("pve_highest_score"),
            _ => unreachable!(),
        }
    }

    pub fn get_total_play_count_field(&self) -> String {
        match self {
            Self::PvE => String::from("pve_total_play_count"),
            Self::PvP => String::from("pvp_total_play_count"),
            Self::Cartesi => String::from("cartesi_total_play_count"),
            _ => unreachable!(),
        }
    }

    pub fn get_elo_field(&self) -> String {
        match self {
            Self::PvP => String::from("pvp_elo_score"),
            Self::Cartesi => String::from("cartesi_elo_score"),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum MatchResult {
    Waiting,
    Playing,
    CodeNotFound,
}

impl Default for MatchResult {
    fn default() -> Self {
        MatchResult::Waiting
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomStatus {
    pub room_id: Uuid,
    pub private_code: String,
    pub match_result: MatchResult,
}

impl RoomStatus {
    pub fn set_error(e: ServerError) -> Self {
        RoomStatus {
            room_id: Default::default(),
            private_code: Default::default(),
            match_result: match e {
                ServerError::RoomNotFound => MatchResult::CodeNotFound,
                ServerError::RoomIsFull => MatchResult::Playing,
                _ => unreachable!(),
            },
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LoginStatus {
    pub need_tutorial: bool,
    pub redirect_to_game_mode: Option<GameMode>,
    pub room_status: RoomStatus,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LoginResponse {
    pub status: LoginStatus,
    pub profile: UserProfile,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PartyCharacterStatusV2 {
    pub requester_id: String,
    pub rival_id: String,
    pub requester_char_uuid_list: Vec<Uuid>,
    pub rival_char_uuid_list: Vec<Uuid>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PartyCharacterStatus {
    pub gamer_addr: String,
    pub char_uuid_list: Vec<Uuid>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PartyCharacterResponse {
    pub requester_id: String,
    pub requester_name: String,
    pub rival_id: String,
    pub rival_name: String,
    pub requester_characters: Vec<CharacterV2>,
    pub rival_characters: Vec<CharacterV2>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PlayerPartyCharacterResponse {
    pub player_addr: String,
    pub player_name: String,
    pub player_characters: Vec<CharacterV2>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EnemyPartyCharacterResponse {
    pub stage_lv: u32,
    pub enemy_addr: String,
    pub enemy_name: String,
    pub enemy_characters: Vec<CharacterV2>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RoomManagerState {
    pub user_to_room: HashMap<String, Uuid>,
    pub room_data: HashMap<Uuid, Room>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoomManager {
    room_map: HashMap<Uuid, Room>,                    // room uuid -> Room
    config_map: HashMap<Uuid, GameplayConfigManager>, // room uuid -> config
    enemy_script_map: HashMap<Uuid, EnemyScriptMap>,  // room uuid -> enemy script
    player_map: HashMap<String, Uuid>,                // player name -> room uuid
    private_map: HashMap<String, Uuid>,               // private code -> room uuid
    reward_cache: HashMap<String, RewardCache>,       // player name -> reward cache
}

impl RoomManager {
    pub fn new() -> Self {
        RoomManager {
            room_map: HashMap::<Uuid, Room>::new(),
            config_map: HashMap::<Uuid, GameplayConfigManager>::new(),
            enemy_script_map: HashMap::<Uuid, EnemyScriptMap>::new(),
            player_map: HashMap::<String, Uuid>::new(),
            private_map: HashMap::<String, Uuid>::new(),
            reward_cache: HashMap::<String, RewardCache>::new(),
        }
    }

    /// Test feature
    pub fn list_all_room(&self) -> Vec<(Uuid, String, Vec<Gamer>)> {
        self.room_map
            .iter()
            .map(|(_, room)| (room.uuid, room.private_code.clone(), room.gamers.clone()))
            .collect::<Vec<(Uuid, String, Vec<Gamer>)>>()
    }

    /// Test feature
    pub fn list_all_player(&self) -> Vec<(&String, &Uuid)> {
        self.player_map
            .iter()
            .map(|(player, uuid)| (player, uuid))
            .collect::<Vec<(&String, &Uuid)>>()
    }

    /// Test feature
    #[cfg(feature = "debug_tool")]
    pub fn import_testing_board(
        &mut self,
        uuid: &Uuid,
        test_board_name: &str,
    ) -> Result<Room, ServerError> {
        use std::fs::File;
        use std::io::Read;
        let config_path = TEST_BOARD_PATH.to_string() + test_board_name;
        let mut f = File::open(&config_path).or(Err(ServerError::InvalidFilePath))?;

        let mut file_data = String::new();
        f.read_to_string(&mut file_data)
            .or(Err(ServerError::InvalidFilePath))?;
        let import_board_data: [[u32; 14]; 5] =
            serde_json::from_str(&file_data).or(Err(ServerError::InvalidJson))?;
        log::debug!("   board content: {:?}", import_board_data);

        // Replacing to test board
        match self.get_room(uuid) {
            Some(room) => {
                let mut next_room = room.clone();
                if let Err(e) = next_room.replace_import_board(&import_board_data) {
                    log::error!("    InternalServerError: \"`{}\"", e.to_string());
                }
                self.update_room(&uuid, &next_room);
                Ok(next_room)
            }
            None => {
                log::error!(
                    "    InternalServerError: \"{}\"",
                    ServerError::RoomNotFound.to_string()
                );
                Err(ServerError::RoomNotFound)
            }
        }
    }

    pub fn get_room(&self, uuid: &Uuid) -> Option<&Room> {
        self.room_map.get(uuid)
    }

    pub fn get_uuid_by_player(&self, player: &str) -> Option<&Uuid> {
        self.player_map.get(player)
    }

    pub fn get_uuid_by_private_code(&self, private_code: &str) -> Option<&Uuid> {
        self.private_map.get(private_code)
    }

    pub fn get_room_status(&self, player: &str) -> Option<RoomStatus> {
        let room_uuid = self.get_uuid_by_player(player)?;
        let room = self.get_room(room_uuid)?;

        let room_status = RoomStatus {
            room_id: room.uuid,
            private_code: room.private_code.clone(),
            match_result: if room.gamers.len() == 2 {
                MatchResult::Playing
            } else {
                MatchResult::Waiting
            },
        };

        log::debug!("    PRIVATE CODE: \"{}\"", room_status.private_code);
        log::debug!("    ROOM ID: {}", room_status.room_id);
        log::debug!("    MATCH RESULT: {:?}", room_status.match_result);

        Some(room_status)
    }

    pub fn get_player_game_mode(&self, player_id: &str) -> Option<GameMode> {
        let room_uuid = self.get_uuid_by_player(player_id)?;
        let room = self.get_room(room_uuid)?;
        Some(room.game_mode)
    }

    pub fn get_dungeon_stage_lv(&self, player_addr: &str) -> Result<u32, DazzleError> {
        let room = {
            let uuid = self
                .get_uuid_by_player(player_addr)
                .ok_or(ServerError::UserNotFound)?;

            self.get_room(uuid).ok_or(ServerError::RoomNotFound)?
        };

        Ok(room.get_dungeon_stage_lv()?.unwrap_or_default())
    }

    /// If player in room, redirect to room. Otherwise it will be treated as normal login.
    pub fn get_login_status(&self, player: &str, need_tutorial: bool) -> LoginStatus {
        match self.get_room_status(player) {
            Some(room_status) => {
                log::debug!("    player already in room, need redirect");
                LoginStatus {
                    need_tutorial,
                    redirect_to_game_mode: self.get_player_game_mode(player),
                    room_status,
                }
            }
            None => {
                log::debug!("    normal login");
                LoginStatus {
                    need_tutorial,
                    redirect_to_game_mode: Default::default(),
                    room_status: Default::default(),
                }
            }
        }
    }

    pub fn create_tutorial_room(
        &mut self,
        player_id: &str,
        tutorial_rival_id: &str,
        player_character_list: &[CharacterV2],
        rival_character_list: &[CharacterV2],
        config_manager: Option<&GameplayConfigManager>,
        seed: u64,
    ) -> Result<RoomStatus, ServerError> {
        // If the tutorial has been interrupted before, delete the old room and create a new one to restart the tutorial.
        if let Some(room_id) = self.get_uuid_by_player(player_id).cloned() {
            if let Some(GameMode::Tutorial) = self.get_player_game_mode(player_id) {
                self.force_remove_room(&room_id)?;
                log::debug!("    Interrupted ROOM removed: {} ", room_id);
            } else {
                return Err(ServerError::InvalidRequest);
            }
        }

        let mut config = config_manager
            .cloned()
            .unwrap_or_else(GameplayConfigManager::new);

        // In the current tutorial SPEC, the player's characters only have "Damage" skill.
        // Setting the energy value slightly below the amount required to use the skill.
        // (E - 1) / (E * max_stack) * 100%
        let cd_rate = ((SkillInfo::Damage.get_config_energy_per_cast() - 1) as f64
            / ((SkillInfo::Damage.get_config_energy_per_cast()
                * SkillInfo::Damage.get_config_max_stack()) as f64)
            * 100.0)
            .round() as u32;

        config.set_char_game_init_cd_rate(cd_rate);

        let mut new_room = Room::new(None, GameMode::Tutorial, None);
        new_room.set_player(
            player_id,
            player_character_list,
            "0",
            &config,
            Some(seed),
            None,
            None,
        );
        new_room.set_player(
            tutorial_rival_id,
            rival_character_list,
            "0",
            &config,
            Some(seed),
            None,
            None,
        );

        let room_status = RoomStatus {
            room_id: new_room.uuid,
            private_code: Default::default(),
            match_result: MatchResult::Playing,
        };

        // "tutorial_rival_id" will not insert into "player_map".
        self.insert_mapping_data(
            new_room.uuid,
            new_room,
            &player_id,
            None,
            config_manager,
            None,
        );

        Ok(room_status)
    }

    pub fn create_pve_room(
        &mut self,
        player_id: &str,
        player_party_characters: &[CharacterV2],
        enemy_party_characters: &[CharacterV2],
        config_manager: Option<&GameplayConfigManager>,
        enemy_script_map: Option<&EnemyScriptMap>,
        seed: Option<u64>,
    ) -> Result<RoomStatus, ServerError> {
        let config = config_manager
            .cloned()
            .unwrap_or_else(GameplayConfigManager::new);

        let mut new_room = Room::new(None, GameMode::PvE, None);

        new_room.set_player(
            player_id,
            player_party_characters,
            "0",
            &config,
            seed,
            None,
            None,
        );
        new_room.set_player(
            ENEMY_ADDR,
            enemy_party_characters,
            "0",
            &config,
            seed,
            None,
            None,
        );

        let room_status = RoomStatus {
            room_id: new_room.uuid,
            private_code: Default::default(),
            match_result: MatchResult::Playing,
        };

        self.insert_mapping_data(
            new_room.uuid,
            new_room,
            &player_id,
            None,
            config_manager,
            enemy_script_map,
        );

        Ok(room_status)
    }

    //#TODO: Need to handle GameMode::DungeonRBS for Prototype2
    pub fn create_dungeon_room(
        &mut self,
        player_id: &str,
        player_party_characters: &[CharacterV2],
        enemy_party_characters: &[CharacterV2],
        dungeon_details: DungeonDetails,
        dungeon_lv: u32,
        stage_lv: u32,
        config_manager: Option<&GameplayConfigManager>,
        enemy_script_map: Option<&EnemyScriptMap>,
        seed: Option<u64>,
    ) -> Result<RoomStatus, ServerError> {
        let config = config_manager
            .cloned()
            .unwrap_or_else(GameplayConfigManager::new);

        let mut new_room = Room::new(None, GameMode::DungeonRBS, Some(dungeon_details));

        new_room.set_player(
            player_id,
            player_party_characters,
            "0",
            &config,
            seed,
            Some(dungeon_lv),
            Some(stage_lv),
        );
        new_room.set_player(
            ENEMY_ADDR,
            enemy_party_characters,
            "0",
            &config,
            seed,
            Some(dungeon_lv),
            Some(stage_lv),
        );

        let room_status = RoomStatus {
            room_id: new_room.uuid,
            private_code: Default::default(),
            match_result: MatchResult::Playing,
        };

        self.insert_mapping_data(
            new_room.uuid,
            new_room,
            &player_id,
            None,
            config_manager,
            enemy_script_map,
        );

        Ok(room_status)
    }

    /// Find a single player room to join, otherwise create a new room.
    pub fn find_room(
        &mut self,
        player: &str,
        party_characters: &[CharacterV2],
        config_manager: Option<&GameplayConfigManager>,
        seed: Option<u64>,
    ) -> Result<(RoomStatus, Option<Vec<String>>), ServerError> {
        let config = config_manager
            .cloned()
            .unwrap_or_else(GameplayConfigManager::new);

        let mut participants_to_increment_game_count: Option<Vec<String>> = None;

        // Is there any single player room?
        // ###TODO: Linear search, should be optimize?
        let (matched_uuid, matched_room) = match self
            .room_map
            .iter()
            .find(|(_, v)| v.gamers.len() < 2 && v.private_code.is_empty())
        {
            // YES, join
            Some((uuid, r)) => {
                log::debug!("    Join room, Init game");
                let mut room = r.clone();
                room.set_player(player, &party_characters, "0", &config, seed, None, None);

                // Postgres DB update should be done here.
                // However, due to dependency issues, we are unable to directly operate the DB at this point.
                // Instead, we will return a flag to the caller, indicating the need to update the DB.
                participants_to_increment_game_count = Some(room.get_participants_id());

                (*uuid, room)
            }
            // NO, create a new room
            None => {
                log::debug!("    Create new room");
                let mut new_room = Room::new(None, GameMode::PvP, None);
                new_room.set_player(player, &party_characters, "0", &config, seed, None, None);
                (new_room.uuid, new_room)
            }
        };

        // compose response
        let room_status = RoomStatus {
            room_id: matched_uuid,
            private_code: matched_room.private_code.clone(),
            match_result: if matched_room.gamers.len() == 2 {
                MatchResult::Playing
            } else {
                MatchResult::Waiting
            },
        };

        self.insert_mapping_data(
            matched_uuid,
            matched_room,
            &player,
            None,
            config_manager,
            None,
        );

        Ok((room_status, participants_to_increment_game_count))
    }

    pub fn create_private_room(
        &mut self,
        player: &str,
        character_list: &[CharacterV2],
        config_manager: Option<&GameplayConfigManager>,
        game_mode: GameMode,
        seed: Option<u64>,
    ) -> Result<RoomStatus, ServerError> {
        if character_list.is_empty() {
            return Err(ServerError::InvalidRequest);
        }

        let config = config_manager
            .cloned()
            .unwrap_or_else(GameplayConfigManager::new);

        let private_code = self.gen_unique_random_id(&self.private_map, PRIVATE_CODE_LENGTH);
        let mut new_room = Room::new(Some(private_code.clone()), game_mode, None);
        new_room.set_player(player, &character_list, STAKE, &config, seed, None, None);

        let room_status = RoomStatus {
            room_id: new_room.uuid,
            private_code: private_code.clone(),
            match_result: MatchResult::Waiting,
        };

        self.insert_mapping_data(
            new_room.uuid,
            new_room,
            player,
            Some(private_code),
            config_manager,
            None,
        );

        Ok(room_status)
    }

    pub fn join_private_room(
        &mut self,
        player: &str,
        private_code: &str,
        character_list: &[CharacterV2],
        config_manager: Option<&GameplayConfigManager>,
        seed: Option<u64>,
    ) -> Result<(RoomStatus, Option<Vec<String>>), ServerError> {
        if character_list.is_empty() {
            return Err(ServerError::InvalidRequest);
        }

        let private_code = private_code.to_uppercase();
        let uuid = self
            .get_uuid_by_private_code(&private_code)
            .ok_or(ServerError::RoomNotFound)?;

        let mut room = self
            .get_room(uuid)
            .ok_or(ServerError::RoomNotFound)?
            .clone();

        // Check room is vacant
        if room.gamers.len() == 2 {
            return Err(ServerError::RoomIsFull);
        }

        let config = config_manager
            .cloned()
            .unwrap_or_else(GameplayConfigManager::new);

        log::debug!("    Join private room, Init game");
        room.set_player(&player, &character_list, STAKE, &config, seed, None, None);
        let room_status = RoomStatus {
            room_id: *uuid,
            private_code: private_code.clone(),
            match_result: MatchResult::Playing,
        };

        // Postgres DB update should be done here.
        // However, due to dependency issues, we are unable to directly operate the DB at this point.
        // Instead, we will return a flag to the caller, indicating the need to update the DB.
        let participants_to_increment_game_count: Option<Vec<String>> =
            Some(room.get_participants_id());

        self.insert_mapping_data(
            room.uuid,
            room,
            player,
            Some(private_code),
            config_manager,
            None,
        );

        Ok((room_status, participants_to_increment_game_count))
    }

    #[cfg(feature = "debug_tool")]
    pub fn setup_debug_room(&mut self, private_code: &str) -> Result<RoomStatus, ServerError> {
        let private_code = private_code.to_uppercase();
        let uuid = self
            .get_uuid_by_private_code(&private_code)
            .ok_or(ServerError::RoomNotFound)?
            .clone();

        let mut room = self
            .get_room(&uuid)
            .ok_or(ServerError::RoomNotFound)?
            .clone();

        log::debug!("Setup debug mode game room");
        room.game_mode = GameMode::Debug;
        let room_status = RoomStatus {
            room_id: uuid,
            private_code: private_code.clone(),
            match_result: MatchResult::Playing,
        };
        self.room_map.insert(uuid, room);
        Ok(room_status)
    }

    /// Canceling match room. If the game already started, the cancel request will be reject and return an error.
    pub fn cancel_room(&mut self, player_id: &str) -> Result<(), ServerError> {
        let room = {
            let uuid = self
                .get_uuid_by_player(player_id)
                .ok_or(ServerError::RoomNotFound)?;

            self.get_room(uuid)
                .ok_or(ServerError::RoomNotFound)?
                .clone()
        };

        // If game already started, reject the cancel request
        if room.gamers.len() == 2 {
            return Err(ServerError::CancelStartedRoom);
        }

        self.force_remove_room(&room.uuid)?;
        log::debug!("    ROOM removed: {} ", room.uuid);
        Ok(())
    }

    /// Must be called while `game_over_result` has winner, or it will return an `InvalidRequest` error.
    pub fn get_room_result(
        &mut self,
        player_id: &str,
        is_tutorial: bool,
        config_manager: Option<&GameplayConfigManager>,
    ) -> Result<(Uuid, GameResult), DazzleError> {
        let room = {
            let uuid = self
                .get_uuid_by_player(player_id)
                .ok_or(ServerError::RoomNotFound)?;

            self.get_room(uuid)
                .ok_or(ServerError::RoomNotFound)?
                .clone()
        };

        let (game_over_result, reward, score_record, reward_cache) = if is_tutorial {
            // Tutorial will not check these results
            (
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            )
        } else {
            if room.gamers.len() != 2 || !room.is_finished() {
                return Err(DazzleError::ServerError(ServerError::InvalidRequest));
            }

            let game_over_result = room
                .get_game_over_result()
                .map_err(|_| ServerError::InvalidRequest)?;

            let reward = room
                .get_game_reward()
                .map_err(|_| ServerError::InvalidRequest)?;

            let config = config_manager
                .cloned()
                .unwrap_or_else(GameplayConfigManager::new);

            let score_record = room
                .cal_score_result(player_id, &config)
                .map_err(|_| ServerError::UserNotFound)?;
            log::debug!("{:#?}", score_record);

            let reward_cache = self
                .get_reward_cache(player_id)
                .cloned()
                .unwrap_or_default();

            (game_over_result, reward, score_record, reward_cache)
        };

        Ok((
            room.uuid,
            GameResult {
                score_record,
                game_over_result,
                reward,
                reward_types: reward_cache.reward_types,
                character_rewards: reward_cache.character_rewards,
                currency_rewards: reward_cache.currency_rewards,
            },
        ))
    }

    pub fn remove_player(&mut self, room_uuid: &Uuid, player: &str) -> Result<(), ServerError> {
        let mut room = self
            .get_room(room_uuid)
            .ok_or(ServerError::RoomNotFound)?
            .clone();

        let gamer = room
            .gamers
            .iter_mut()
            .find(|g| g.id == *player)
            .ok_or(ServerError::UserNotFound)?;
        gamer.is_quit_room = true;

        self.update_room(room_uuid, &room);
        self.remove_player_map(player);

        Ok(())
    }

    /// Get full characters data in both side parties
    pub fn get_both_side_party_status(
        &self,
        requester_id: &str,
    ) -> Result<PartyCharacterStatusV2, ServerError> {
        let room = {
            let uuid = self
                .get_uuid_by_player(requester_id)
                .ok_or(ServerError::UserNotFound)?;

            self.get_room(uuid).ok_or(ServerError::RoomNotFound)?
        };

        let requester_char_uuid_list = room
            .gamers
            .iter()
            .find(|g| g.id == requester_id)
            .ok_or(ServerError::UserNotFound)?
            .character_uuid_list
            .clone();

        let rival = room
            .gamers
            .iter()
            .find(|g| g.id != requester_id)
            .ok_or(ServerError::UserNotFound)?;

        Ok(PartyCharacterStatusV2 {
            requester_id: requester_id.to_owned(),
            rival_id: rival.id.clone(),
            requester_char_uuid_list,
            rival_char_uuid_list: rival.character_uuid_list.clone(),
        })
    }

    pub fn get_party_status(
        &self,
        gamer_addr: &str,
        gamer_type: DungeonGamer,
    ) -> Result<PartyCharacterStatus, ServerError> {
        let room = {
            let uuid = self
                .get_uuid_by_player(gamer_addr)
                .ok_or(ServerError::UserNotFound)?;

            self.get_room(uuid).ok_or(ServerError::RoomNotFound)?
        };

        let opt_gamer = match gamer_type {
            DungeonGamer::Player => room.gamers.iter().find(|g| g.id == gamer_addr),
            DungeonGamer::Enemy => room.gamers.iter().find(|g| g.id != gamer_addr),
        };

        let char_uuid_list = opt_gamer
            .ok_or(ServerError::UserNotFound)?
            .character_uuid_list
            .clone();

        Ok(PartyCharacterStatus {
            gamer_addr: gamer_addr.to_owned(),
            char_uuid_list,
        })
    }

    // For room matching
    fn insert_mapping_data(
        &mut self,
        uuid: Uuid,
        room: Room,
        player: &str,
        private_code: Option<String>,
        config_manager: Option<&GameplayConfigManager>,
        enemy_script_map: Option<&EnemyScriptMap>,
    ) {
        self.room_map.insert(uuid, room);
        self.player_map.insert(player.to_owned(), uuid);

        if let Some(code) = private_code {
            self.private_map.insert(code, uuid);
        };

        // In debug mode it might causes concurrentcy issue during match.
        // To simplify this situation, always use the config settings that were applied when the room was created.
        if !self.config_map.contains_key(&uuid) {
            let config = config_manager
                .cloned()
                .unwrap_or_else(GameplayConfigManager::new);
            self.config_map.insert(uuid, config);
        }

        //###TODO: Will be remove and merge to enemy_template
        if let Some(script_map) = enemy_script_map {
            self.enemy_script_map.insert(uuid, script_map.clone());
        }
    }

    /// Return true if the room has actually been remved.
    ///
    /// If any player not quit, remove operation will be skip.
    pub fn remove_empty_room(&mut self, room_uuid: &Uuid) -> Result<bool, ServerError> {
        let room = self.get_room(room_uuid).ok_or(ServerError::RoomNotFound)?;

        // The room can only be removed when it is confirmed that both players have left the game.
        if !room.is_all_players_quit() {
            log::debug!(
                "Not all players quit the room, abort to remove room: {}",
                room_uuid
            );
            return Ok(false);
        }

        if !room.private_code.is_empty() {
            self.private_map.remove(&room.private_code.clone());
        }

        self.room_map.remove(room_uuid);
        self.config_map.remove(room_uuid);
        self.enemy_script_map.remove(room_uuid);

        Ok(true)
    }

    /// Remove the room and clear all existed players
    pub fn force_remove_room(&mut self, room_uuid: &Uuid) -> Result<(), ServerError> {
        let room = self.get_room(room_uuid).ok_or(ServerError::RoomNotFound)?;
        let gamers = room.get_gamers_id();

        if !room.private_code.is_empty() {
            self.private_map.remove(&room.private_code.clone());
        }

        gamers
            .iter()
            .for_each(|gamer| self.remove_player_map(&gamer));

        self.room_map.remove(room_uuid);
        self.config_map.remove(room_uuid);
        self.enemy_script_map.remove(room_uuid);

        Ok(())
    }

    fn remove_player_map(&mut self, player: &str) {
        self.player_map.remove(player);
    }

    pub fn remove_reward_character_uuid(&mut self, player_addr: &str) -> Result<(), ServerError> {
        match self
            .get_uuid_by_player(&player_addr)
            .and_then(|uuid| self.get_room(uuid))
        {
            Some(r) => {
                let mut room = r.clone();
                room.remove_reward_character_uuid();
                self.update_room(&room.uuid, &room);
                Ok(())
            }
            None => Err(ServerError::RoomNotFound),
        }
    }

    // For update room data during gameplay (move, active skill)
    pub fn update_room(&mut self, uuid: &Uuid, room: &Room) {
        self.room_map.insert(uuid.clone(), room.clone());
    }

    pub fn move_action(
        &mut self,
        room_uuid: &Uuid,
        player: &str,
        action: &MoveAction,
        attacker_id: &Uuid,
        defender_id: &Uuid,
    ) -> Result<Room, DazzleError> {
        let config = self
            .config_map
            .get(room_uuid)
            .ok_or(ServerError::ConfigNotFound)?;

        match self.get_room(room_uuid) {
            Some(room) => {
                let mut room = room.clone();
                room.check_mover(player)?;
                room.check_legal_move(&action)?;

                room.update_game(
                    room.game.current_active_player_idx,
                    action,
                    attacker_id,
                    defender_id,
                    config,
                )?;

                if matches!(room.game_mode, GameMode::PvE | GameMode::DungeonRBS)
                    && room.game_over_result.is_none()
                {
                    let enemy_script_map = self
                        .enemy_script_map
                        .get(room_uuid)
                        .ok_or(ServerError::EnemyScriptNotFound)?;

                    room.update_enemy_turn(
                        room.game.current_active_player_idx,
                        config,
                        enemy_script_map,
                    )?;
                }

                self.update_room(&room.uuid, &room);
                Ok(room)
            }
            None => Err(ServerError::RoomNotFound.into()),
        }
    }

    pub fn skill_action(
        &mut self,
        room_uuid: &Uuid,
        player: &str,
        caster_id: Uuid,
        ally_target_id: Uuid,
        rival_target_id: Option<Uuid>,
    ) -> Result<Room, DazzleError> {
        let config = self
            .config_map
            .get(room_uuid)
            .ok_or(ServerError::ConfigNotFound)?;

        match self.get_room(room_uuid) {
            Some(room) => {
                let mut room = room.clone();
                room.check_mover(player)?;

                room.activate_skill(
                    room.game.current_active_player_idx,
                    caster_id,
                    ally_target_id,
                    rival_target_id,
                    config,
                )?;

                self.update_room(&room.uuid, &room);
                Ok(room)
            }
            None => Err(ServerError::RoomNotFound.into()),
        }
    }

    pub fn quit_game(&mut self, player: &str) -> Result<Room, ServerError> {
        let mut room = {
            let uuid = self
                .get_uuid_by_player(player)
                .ok_or(ServerError::RoomNotFound)?;

            self.get_room(uuid)
                .ok_or(ServerError::RoomNotFound)?
                .clone()
        };

        room.set_game_forfeit(player)?;

        self.update_room(&room.uuid, &room);

        Ok(room)
    }

    pub fn update_room_rng(&mut self, uuid: &Uuid, new_rng_seed: u64) -> Result<Room, DazzleError> {
        match self.get_room(uuid) {
            Some(room) => {
                let mut room = room.clone();
                room.game.update_rng(new_rng_seed);

                self.update_room(&room.uuid, &room);
                Ok(room)
            }
            None => {
                log::error!(
                    "    InternalServerError: \"{}\"",
                    ServerError::RoomNotFound.to_string()
                );
                Err(ServerError::RoomNotFound.into())
            }
        }
    }

    fn gen_unique_random_id(&self, private_map: &HashMap<String, Uuid>, length: usize) -> String {
        let char_white_list: Vec<char> = "23456789ABCDEFGHJKMNPQRSTUVWXYZ"
            .to_string()
            .chars()
            .collect();
        let mut private_code = self.generate_code(length, &char_white_list);
        // Check duplicated
        while let Some(_) = private_map.get(&private_code) {
            private_code = self.generate_code(length, &char_white_list);
        }
        private_code
    }

    fn generate_code(&self, length: usize, charset: &[char]) -> String {
        let mut code = String::new();
        let mut rng = rand::thread_rng();
        let die = Uniform::from(0..charset.len());
        for _ in 0..length {
            code.push(charset[die.sample(&mut rng)]);
        }
        code
    }

    pub fn get_current_state(&self) -> RoomManagerState {
        let user_to_room = self
            .player_map
            .iter()
            .map(|(name, uuid)| (name.to_lowercase(), uuid.clone()))
            .collect();

        let room_snapshots = self
            .room_map
            .iter()
            .map(|(_, room)| (room.uuid, room.snapshot()))
            .collect();

        RoomManagerState {
            user_to_room,
            room_data: room_snapshots,
        }
    }

    pub fn insert_reward_cache(&mut self, player: String, rc: RewardCache) {
        self.reward_cache.insert(player, rc);
    }

    pub fn get_reward_cache(&self, player: &str) -> Option<&RewardCache> {
        self.reward_cache.get(player)
    }

    pub fn remove_reward_cache(&mut self, player: &str) {
        self.reward_cache.remove(player);
    }

    pub fn end_dungeon_rbs_game(&mut self, player: &str) -> Result<(), ServerError> {
        let room_uuid = match self.get_uuid_by_player(player) {
            Some(uuid) => uuid.to_owned(),
            None => {
                return Ok(());
            }
        };

        let room = match self.get_room(&room_uuid) {
            Some(r) => r,
            None => {
                return Ok(());
            }
        };

        if room.game_mode != GameMode::DungeonRBS {
            return Ok(());
        }

        //#HACK: For testing rewarding prototype1 & prototype2, this function will end the game when gameMode == GameMode::DungeonRBS

        let mut new_room = room.clone();

        new_room.set_game_result(0, player, false).map_err(|e| {
            log::error!("    InternalServerError: \"{}\"", e.to_string());
            ServerError::InvalidRequest
        })?;

        self.update_room(&room_uuid, &new_room);
        Ok(())
    }
}
