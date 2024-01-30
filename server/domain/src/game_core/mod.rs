pub mod board;
pub mod character;
pub mod character_mod;
pub mod config;
pub mod event_module;
pub mod game;
pub mod probability_mod;
pub mod reward;
pub mod room_manager;
pub mod skill;
pub mod users;

use atb::prelude::*;

#[derive(thiserror::Error, Debug)]
pub enum ServerError {
    #[error("Room not found")]
    RoomNotFound,

    #[error("Config not found")]
    ConfigNotFound,

    #[error("Enemy script not found")]
    EnemyScriptNotFound,

    #[error("Room is full")]
    RoomIsFull,

    #[error("User not found")]
    UserNotFound,

    #[error("Can't cancel started room")]
    CancelStartedRoom,

    #[error("Invalid request")]
    InvalidRequest,

    #[error("Invalid file path")]
    InvalidFilePath,

    #[error("JSON was not well-formatted")]
    InvalidJson,

    #[error("Invalid hex payload")]
    InvalidHex,

    #[error("Invalid wallet address: {0}")]
    InvalidAddress(String),

    #[error("Invalid currency number: {0}")]
    InvalidCurrency(String),

    #[error("Insufficient balance {0} to withdraw {1}")]
    InsufficientBalance(String, String),

    #[error("Insufficient tournament fee - balance: {0}, stake: {1}")]
    InsufficientTournamentFee(String, String),

    #[error("Invalid ingame-wallet: {0}")]
    InvalidIngameWallet(String),

    #[error("Failed to build http request")]
    FailedToBuildRequest,

    #[error("Failed to handle http response")]
    FailedToHandleResponse,

    #[error("Failed to send notice")]
    FailedToSendNotice,

    #[error("Failed to send report")]
    FailedToSendReport,

    #[error("Invalid abi-encoded data")]
    InvalidABIData,

    #[error("Graphql query failed")]
    GraphqlQueryFailed,

    #[error("Cartesi inspect api call failed")]
    CartesiInspectApiFailed,

    #[error("Retry connection has failed {0} times")]
    RetryConnectionAndFailed(u8),

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Failed to connect to Ethereum node")]
    FailedToConnectEtherNode,

    #[error("Failed to load ethereum smart contract")]
    FailedToLoadContract,

    #[error("Error occurred while contract query")]
    ContractQueryFailed,

    #[error("Error occurred while contract call")]
    ContractCallFailed,

    #[error("Invalid smart contract event")]
    InvalidContractEvent,

    #[error("Invalid Uuid")]
    InvalidUuid,

    #[error("Character not found")]
    CharacterNotFound,

    #[error("Insert new character failed: {0}")]
    InsertCharacterFailed(String),

    #[error("Request for a character: {0} that is not owned by user on chain")]
    RequestNotOwnedNFT(String),

    #[error("Failed to query current block number")]
    FailedToFetchBlockNumber,

    #[error("Invalid NFT")]
    InvalidNFT,

    #[error("Failed to load NFT ownership data from file system")]
    FailedToLoadNFTData,

    #[error("Failed to save NFT ownership data to file system")]
    FailedToSaveNFTData,

    #[error("Failed to update DB")]
    FailedToUpdateDB,

    #[error("Failed to query DB")]
    FailedToQueryDB,

    #[error("Failed to connect DB")]
    FailedToConnectDB,

    #[error("Invalid config parameters")]
    InvalidConfigParam,

    #[error("Insufficient ingame-currency")]
    InsufficientIngameCurrency,
}

#[derive(thiserror::Error, Debug)]
pub enum GameError {
    #[error("user not found")]
    UserNotFound,

    #[error("character not found")]
    CharacterNotFound,

    #[error("character element error")]
    CharacterElementError,

    #[error("Enemy script not found: {0}")]
    EnemyScriptNotFound(String),

    #[error("game not start")]
    NoGameState,

    #[error("damage evaluating error")]
    DamageEvaluate,

    #[error("invalid input `{0}")]
    InvalidInput(String),

    #[error("invalid operaton")]
    InvalidOperation,

    #[error("skill not ready to use")]
    SkillNotReady,

    #[error("skill parameter error")]
    SkillParamError,

    #[error("No available target.")]
    SkillNoGemToTrigger,

    #[error("Illegal move or action")]
    IllegalMove,

    #[error("Create next stage failed")]
    CreateDungeonStageFailed,

    #[error("Fetch next stage failed")]
    FetchDungeonStageFailed,

    #[error("Dungeon details not found")]
    DungeonNotFound,
}

#[derive(thiserror::Error, Debug)]
pub enum DinderError {
    #[error("GameError: {0}")]
    GameError(#[from] GameError),
    #[error("ServerError: {0}")]
    ServerError(#[from] ServerError),
}
