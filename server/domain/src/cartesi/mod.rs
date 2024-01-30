use crate::game_core::game::Room;
use atb_types::prelude::uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::{Display as StrumDisplay, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResponse {
    pub index: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DinderNotice {
    pub notice_type: NoticeType,
    pub base64_content: String,
    pub user: String,
    pub balance: Option<String>, //#NOTE: if no balance update, it will return None
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DinderReport {
    pub error_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voucher {
    //#NOTE: this should be the contract address to handle the voucher
    pub destination: String,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoucherMeta {
    pub timestamp: u64,
    pub input_index: String,
    pub amount: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvanceMetadata {
    pub msg_sender: String,
    pub input_index: u64,
    pub block_number: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvanceRequest {
    pub metadata: Option<AdvanceMetadata>,
    /*
        #Note: :
        This is actually the data we passed in through input
        We'll later convert payload into GameReqest object
    */
    pub payload: String,
}

/*
    Example: {"operation":"create_private_room","data":"{\"user\":\"Test\"}"}
*/
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameRequest {
    //#NOTE: must be one of the DinderOperation
    pub operation: String,
    //#NOTE: hex-encoded Dinder json request (FindRoomRequest, CreatePrivateRoomRequest, JoinPrivateRoomRequest, CancelRoomRequest, ActiveSkillsRequest, MoveRequest, WithdrawRequest)
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, StrumDisplay, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum DinderOperation {
    CreatePrivateRoom,
    JoinPrivateRoom,
    CancelRoom,
    GameOver,
    Move,
    ActivateSkill,
    QuitGame,
    TransferBalance,
    AttachIngameWallet,
}

#[derive(Debug, Clone, Serialize, StrumDisplay, EnumString, Deserialize)]
#[strum(serialize_all = "snake_case")]
pub enum NoticeType {
    Room,
    CancelRoom,
    GameResult,
    Deposit,
    Transfer,
    AttachIngameWallet,
    Error, //#TODO: we'll generate ErrorNotice to record that there is error occurred in Cartesi dapp, but we need to accpet all the input
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectResponse {
    pub user_to_room: HashMap<String, Uuid>,
    pub balance: HashMap<String, String>,
    pub voucher_meta: HashMap<String, Vec<VoucherMeta>>,
    pub room_data: HashMap<Uuid, Room>,
    pub ingame_wallets: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InspectResponseWrapper {
    pub status: String,
    pub exception_payload: Option<String>,
    pub reports: Vec<Report>,
    pub processed_input_count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, StrumDisplay, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum FinishStatus {
    Accept,
    Reject,
}

#[derive(Debug, Clone, Deserialize, Serialize, StrumDisplay, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum RequestType {
    AdvanceState,
    InspectState,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RollupResponse {
    pub request_type: String,
    pub data: AdvanceRequest,
}
