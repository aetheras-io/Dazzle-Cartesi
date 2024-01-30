use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_name: String,
    pub portrait_id: u32,
    pub ingame_currency: u32,
}

//#Note: Map to a Unity struct with the same name, but the field naming is using snake_case
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankProfile {
    pub address: String,
    pub player_id: String,
    pub portrait_id: u32,
    pub rank: u32,
    pub points: u32,
    pub pve_win_count: u32,
    pub pve_total_play_count: u32,
    pub pvp_win_count: u32,
    pub pvp_total_play_count: u32,
    pub cartesi_win_count: u32,
    pub cartesi_total_play_count: u32,
}
