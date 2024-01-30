use super::balance_manager::BalanceManager;
use super::http_dispatcher::{
    send_finish_request, send_notice, send_report, send_room_snapshot_notice, send_voucher,
};
use super::ingame_wallet_manager::IngameWalletManager;
use atb_types::prelude::uuid::Uuid;
use base64::{engine::general_purpose, Engine as _};
use domain::cartesi::{
    AdvanceMetadata, AdvanceRequest, DazzleOperation, DazzleReport, FinishStatus, GameRequest,
    InspectResponse, NoticeType, RequestType, RollupResponse, VoucherMeta,
};
use domain::game_core::board::MoveAction;
use domain::game_core::character::CharacterV2;
use domain::game_core::config::STAKE;
use domain::game_core::room_manager::*;
use domain::game_core::{DazzleError, ServerError};
use ethers_core::{
    abi::{decode, encode, short_signature, ParamType, Token},
    types::{Address, U256},
    utils::hex,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use std::env;
use std::str::FromStr;

// For request
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FindRoomRequest {
    user: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreatePrivateRoomRequest {
    user: String,
    base64_character_list: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JoinPrivateRoomRequest {
    user: String,
    private_code: String,
    base64_character_list: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct QuitGameRequest {
    user: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TransferRequest {
    from_address: String,
    to_address: String,
    amount: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AttachIngameWalletRequest {
    ingame_wallet_address: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CancelRoomRequest {
    user: String,
}

// #[derive(Debug, Clone, Deserialize, Serialize)]
// pub struct GetCharactersRequest {
//     characters: Vec<i32>,
// }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MoveRequest {
    room_id: Uuid,
    user: String,
    action: MoveAction,
    attacker_id: Uuid,
    defender_id: Uuid,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetRoomEventRequest {
    room_id: Uuid,
    current_state_len: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActiveSkillsRequest {
    room_id: Uuid,
    user: String,
    caster_id: Uuid,
    ally_target_id: Uuid,
    rival_target_id: Option<Uuid>,
}

// #[derive(Debug, Clone, Deserialize, Serialize)]
// struct LoginRequest {
//     pub user: String,
// }

async fn create_private_room(
    room_manager: &mut RoomManager,
    http_dispatcher_url: &str,
    req_data: &[u8],
) -> Result<FinishStatus, DazzleError> {
    log::debug!("CREATE PRIVATE ROOM");
    let req: CreatePrivateRoomRequest = serde_json::from_slice(req_data).map_err(|e| {
        log::debug!("Failed to deserialize CreatePrivateRoomRequest: {}", e);
        ServerError::InvalidRequest
    })?;

    if room_manager.get_room_status(&req.user).is_some() {
        // Cartesi mode support reconnect now, skip create room procedure if player is already in game
        return Ok(FinishStatus::Accept);
    }

    let character_list_bz = general_purpose::STANDARD
        .decode(&req.base64_character_list)
        .map_err(|e| {
            log::debug!(
                "Failed to decode base64 payload: {} - {}",
                req.base64_character_list.clone(),
                e.to_string()
            );
            ServerError::InvalidRequest
        })?;

    let character_list: Vec<CharacterV2> =
        serde_json::from_slice(&character_list_bz).map_err(|e| {
            log::debug!("Failed to deserialize party character data list: {}", e);
            ServerError::InvalidJson
        })?;

    let room_status = room_manager.create_private_room(
        &req.user,
        &character_list,
        None,
        GameMode::Cartesi,
        None,
    )?;
    log::debug!("    PRIVATE CODE: {}", &room_status.private_code);
    log::debug!("    ROOM ID: {}", room_status.room_id.to_owned());

    let new_room = room_manager.get_room(&room_status.room_id).unwrap();
    // let stake_str = new_room.gamers[0].stake.as_ref();
    // let stake = U256::from_dec_str(stake_str)
    //     .map_err(|_| ServerError::InvalidCurrency(stake_str.to_owned()))?;
    // let wallet =
    //     Address::from_str(&req.user).map_err(|_| ServerError::InvalidAddress(req.user.clone()))?;

    //#TODO: when to reset the balance when the tournament is over??
    // let new_balance = balance_manager.withdraw(&wallet, stake)?;
    // let balance_str = new_balance.to_string();

    send_room_snapshot_notice(http_dispatcher_url, &req.user, new_room, None).await
}

async fn join_private_room(
    room_manager: &mut RoomManager,
    http_dispatcher_url: &str,
    req_data: &[u8],
    new_seed: u64,
) -> Result<FinishStatus, DazzleError> {
    let req: JoinPrivateRoomRequest = serde_json::from_slice(req_data).map_err(|e| {
        log::debug!("Failed to deserialize JoinPrivateRoomRequest: {}", e);
        ServerError::InvalidRequest
    })?;
    log::debug!("Join PRIVATE ROOM, code: \"{}\"", &req.private_code);

    //#NOTE: Cartesi mode support reconnect now, reject input if player is already in game
    if room_manager.get_room_status(&req.user).is_some() {
        return Ok(FinishStatus::Reject);
    }

    let character_list_bz = general_purpose::STANDARD
        .decode(&req.base64_character_list)
        .map_err(|e| {
            log::debug!(
                "Failed to decode base64 payload: {} - {}",
                req.base64_character_list.clone(),
                e.to_string()
            );
            ServerError::InvalidRequest
        })?;

    let character_list: Vec<CharacterV2> =
        serde_json::from_slice(&character_list_bz).map_err(|e| {
            log::debug!("Failed to deserialize party character data list: {}", e);
            ServerError::InvalidJson
        })?;

    let (room_status, _) = room_manager.join_private_room(
        &req.user,
        &req.private_code,
        &character_list,
        None,
        None,
    )?;

    log::debug!("    PRIVATE CODE: {}", room_status.private_code.clone());
    log::debug!("    ROOM ID: {}", room_status.room_id.to_owned());

    let new_room = room_manager.update_room_rng(&room_status.room_id, new_seed)?;

    //#TODO: when to reset the balance when the tournament is over??
    // let stake_str = new_room.gamers[1].stake.as_ref();
    // let stake = U256::from_dec_str(stake_str)
    //     .map_err(|_| ServerError::InvalidCurrency(stake_str.to_owned()))?;
    // let wallet =
    //     Address::from_str(&req.user).map_err(|_| ServerError::InvalidAddress(req.user.clone()))?;
    // let new_balance = balance_manager.withdraw(&wallet, stake)?;
    // let balance_str = new_balance.to_string();

    send_room_snapshot_notice(http_dispatcher_url, &req.user, &new_room, None).await
}

async fn cancel_room(
    room_manager: &mut RoomManager,
    http_dispatcher_url: &str,
    req_data: &[u8],
) -> Result<FinishStatus, DazzleError> {
    let req: CancelRoomRequest = serde_json::from_slice(req_data).map_err(|e| {
        log::debug!("Failed to deserialize CancelRoomRequest: {}", e);
        ServerError::InvalidRequest
    })?;

    log::debug!("CANCEL ROOM, user: {}", req.user);

    room_manager.cancel_room(&req.user)?;
    send_notice(
        http_dispatcher_url,
        NoticeType::CancelRoom,
        "",
        &req.user,
        None,
    )
    .await
}

async fn game_over(
    room_manager: &mut RoomManager,
    balance_manager: &mut BalanceManager,
    http_dispatcher_url: &str,
    req_data: &[u8],
) -> Result<FinishStatus, DazzleError> {
    let req: FindRoomRequest = serde_json::from_slice(req_data).map_err(|e| {
        log::debug!("Failed to deserialize FindRoomRequest: {}", e);
        ServerError::InvalidRequest
    })?;

    log::debug!("GAME OVER");

    let uuid = room_manager
        .get_uuid_by_player(&req.user)
        .ok_or(ServerError::RoomNotFound)?
        .clone();

    let (room_uuid, game_result) = room_manager.get_room_result(&req.user, false, None)?;
    room_manager.remove_player(&room_uuid, &req.user)?;
    room_manager.remove_empty_room(&room_uuid)?;

    let address =
        Address::from_str(&req.user).map_err(|_| ServerError::InvalidAddress(req.user.clone()))?;

    let balance = balance_manager
        .get_balance(&address)
        .map_or_else(|| "0".to_owned(), |b| b.to_string());

    if let Some(room) = room_manager.get_room(&uuid) {
        //#NOTE: Since Room has been modified, we need to send a notice, so that CartesiHarvester can maintain the correct projection of the room_data
        send_room_snapshot_notice(http_dispatcher_url, &req.user, room, Some(balance.clone()))
            .await?;
    }

    let game_over_notice = serde_json::to_string(&game_result).unwrap();
    send_notice(
        http_dispatcher_url,
        NoticeType::GameResult,
        &game_over_notice,
        &req.user,
        Some(balance),
    )
    .await
}

async fn action_move(
    room_manager: &mut RoomManager,
    http_dispatcher_url: &str,
    req_data: &[u8],
    new_seed: u64,
) -> Result<FinishStatus, DazzleError> {
    let req: MoveRequest = serde_json::from_slice(req_data).map_err(|e| {
        log::debug!("Failed to deserialize MoveRequest: {}", e);
        ServerError::InvalidRequest
    })?;

    log::debug!(
        "MOVE ACTION {:?}, user: \"{}\"",
        req.action,
        req.user.clone()
    );

    room_manager.update_room_rng(&req.room_id, new_seed)?;

    let room = room_manager.move_action(
        &req.room_id,
        &req.user,
        &req.action,
        &req.attacker_id,
        &req.defender_id,
    )?;

    send_room_snapshot_notice(http_dispatcher_url, &req.user, &room, None).await
}

async fn activate_skill(
    room_manager: &mut RoomManager,
    http_dispatcher_url: &str,
    req_data: &[u8],
    new_seed: u64,
) -> Result<FinishStatus, DazzleError> {
    let req: ActiveSkillsRequest = serde_json::from_slice(req_data).map_err(|e| {
        log::debug!("Failed to deserialize ActiveSkillsRequest: {}", e);
        ServerError::InvalidRequest
    })?;

    log::debug!("ACTIVATE SKILLS, user: \"{}\"", req.user.clone());

    room_manager.update_room_rng(&req.room_id, new_seed)?;

    let room = room_manager.skill_action(
        &req.room_id,
        &req.user,
        req.caster_id,
        req.ally_target_id,
        req.rival_target_id,
    )?;
    log::debug!("Done");
    send_room_snapshot_notice(http_dispatcher_url, &req.user, &room, None).await
}

async fn quit_game(
    room_manager: &mut RoomManager,
    http_dispatcher_url: &str,
    req_data: &[u8],
) -> Result<FinishStatus, DazzleError> {
    let req: QuitGameRequest = serde_json::from_slice(req_data).map_err(|e| {
        log::debug!("Failed to deserialize QuitGameRequest: {}", e);
        ServerError::InvalidRequest
    })?;

    log::debug!("QUIT GAME, user: \"{}\"", req.user);

    let room = room_manager.quit_game(&req.user)?;
    send_room_snapshot_notice(http_dispatcher_url, &req.user, &room, None).await
}

async fn transfer(
    balance_manager: &mut BalanceManager,
    http_dispatcher_url: &str,
    dapp_address: &str,
    metadata: AdvanceMetadata,
    req_data: &[u8],
) -> Result<FinishStatus, DazzleError> {
    let req: TransferRequest =
        serde_json::from_slice(req_data).map_err(|_| ServerError::InvalidRequest)?;

    log::debug!(
        "Transfer balance, from_address: \"{}\", to_address: \"{}\", amount: \"{}\"",
        req.from_address,
        req.to_address,
        req.amount,
    );

    let from_address = Address::from_str(&req.from_address)
        .map_err(|_| ServerError::InvalidAddress(req.from_address.clone()))?;

    let to_address = Address::from_str(&req.to_address)
        .map_err(|_| ServerError::InvalidAddress(req.to_address.clone()))?;

    let amount =
        U256::from_dec_str(&req.amount).map_err(|_| ServerError::InvalidCurrency(req.amount))?;

    let amount_string = amount.to_string();

    // let fee_amount = U256::from_dec_str(WITHDRAWAL_FEE)
    //     .map_err(|_| ServerError::InvalidCurrency(WITHDRAWAL_FEE.to_owned()))?;

    let inner_data = vec![Token::Address(to_address), Token::Uint(amount)];
    let mut encoded_inner = encode(&inner_data);
    let withdrawal_params = vec![ParamType::Address, ParamType::Uint(256)];
    let mut payload_bz = short_signature("withdrawEther", &withdrawal_params).to_vec();
    payload_bz.append(&mut encoded_inner);

    send_voucher(http_dispatcher_url, dapp_address, &payload_bz).await?;

    // let total_amount = amount.saturating_add(fee_amount);

    //#NOTE: if withdraw failed, we'll reject the whole request, and all the generated vouchers and notices will be discarded
    let from_new_balance = balance_manager.withdraw(&from_address, amount)?;
    if let Some(from_voucher_meta_list) = balance_manager.get_voucher_meta(&from_address) {
        let from_voucher_json = serde_json::to_string(from_voucher_meta_list).unwrap();

        send_notice(
            http_dispatcher_url,
            NoticeType::Transfer,
            &from_voucher_json,
            &req.from_address,
            Some(from_new_balance.to_string()),
        )
        .await?;
    } else {
        let from_voucher_json = serde_json::to_string(&Vec::<VoucherMeta>::new()).unwrap();

        send_notice(
            http_dispatcher_url,
            NoticeType::Transfer,
            &from_voucher_json,
            &req.from_address,
            Some(from_new_balance.to_string()),
        )
        .await?;
    }

    //#NOTE: transfer to to_address so the voucher will be given to to_address to let them execute it later
    balance_manager.update_voucher_meta(&to_address, amount_string, metadata);

    // let admin_wallet = Address::from_str(ADMIN_WALLET_ADDRESS)
    //     .map_err(|_| ServerError::InvalidAddress(ADMIN_WALLET_ADDRESS.to_owned()))?;

    let to_new_balance = balance_manager.deposit(&to_address, amount);
    let to_voucher_meta_list = balance_manager.get_voucher_meta(&to_address).unwrap();
    let to_voucher_json = serde_json::to_string(to_voucher_meta_list).unwrap();

    send_notice(
        http_dispatcher_url,
        NoticeType::Transfer,
        &to_voucher_json,
        &req.to_address,
        Some(to_new_balance.to_string()),
    )
    .await
}

pub async fn attach_ingame_wallet(
    ingame_wallet_manager: &mut IngameWalletManager,
    http_dispatcher_url: &str,
    metadata: AdvanceMetadata,
    req_data: &[u8],
) -> Result<FinishStatus, DazzleError> {
    let req: AttachIngameWalletRequest =
        serde_json::from_slice(req_data).map_err(|_| ServerError::InvalidRequest)?;

    let ingame_wallet_str = req.ingame_wallet_address.to_lowercase();
    let metamask_wallet_str = metadata.msg_sender.to_lowercase();

    log::debug!(
        "Register ingame wallet: \"{}\" with metamask : \"{}\"",
        ingame_wallet_str,
        metamask_wallet_str,
    );

    let ingame_wallet_address = Address::from_str(&ingame_wallet_str)
        .map_err(|_| ServerError::InvalidAddress(ingame_wallet_str.clone()))?;

    let metamask_wallet_address = Address::from_str(&metamask_wallet_str)
        .map_err(|_| ServerError::InvalidAddress(metamask_wallet_str.clone()))?;

    ingame_wallet_manager.set_ingame_wallet(&metamask_wallet_address, ingame_wallet_address);

    send_notice(
        http_dispatcher_url,
        NoticeType::AttachIngameWallet,
        &ingame_wallet_str,
        &metamask_wallet_str,
        None,
    )
    .await
}

pub async fn inspect_state(
    room_manager: &RoomManager,
    balance_manager: &BalanceManager,
    ingame_wallet_manager: &IngameWalletManager,
    http_dispatcher_url: &str,
) -> Result<FinishStatus, DazzleError> {
    log::debug!("inspect_state");

    let room_manager_state = room_manager.get_current_state();
    let balance_manager_state = balance_manager.get_current_state();
    let ingame_wallet_manager_state = ingame_wallet_manager.get_current_state();
    let inspect_res = InspectResponse {
        user_to_room: room_manager_state.user_to_room,
        balance: balance_manager_state.balance_map,
        voucher_meta: balance_manager_state.voucher_meta_map,
        room_data: room_manager_state.room_data,
        ingame_wallets: ingame_wallet_manager_state.wallet_map,
    };

    let report_json = serde_json::to_string(&inspect_res).unwrap();
    send_report(http_dispatcher_url, &report_json).await
}

pub async fn handle_deposit(
    http_dispatcher_url: &str,
    balance_manager: &mut BalanceManager,
    bz_payload: &[u8],
) -> Result<FinishStatus, DazzleError> {
    let params = vec![ParamType::Address, ParamType::Uint(256)];

    let decoded = decode(&params, bz_payload).map_err(|_| ServerError::InvalidABIData)?;

    if decoded.len() != 2 {
        return Ok(FinishStatus::Reject);
    }

    let depositer = match decoded[0] {
        Token::Address(address) => address,
        _ => {
            log::debug!("Invalid abi data: sender");
            return Ok(FinishStatus::Reject);
        }
    };

    let deposit_amount = match decoded[1] {
        Token::Uint(amount) => amount,
        _ => {
            log::debug!("Invalid abi data: Ether amount");
            return Ok(FinishStatus::Reject);
        }
    };

    log::debug!("Address: {} deposited {} eth", &depositer, deposit_amount);
    let new_balance = balance_manager.deposit(&depositer, deposit_amount);
    log::debug!("New balance: {} eth", &new_balance);
    let user = format!("{:#x}", depositer);
    send_notice(
        http_dispatcher_url,
        NoticeType::Deposit,
        "",
        &user,
        Some(new_balance.to_string()),
    )
    .await
}

fn auth_msg_sender(
    balance_manager: &BalanceManager,
    ingame_wallet_manager: &IngameWalletManager,
    msg_sender: &str,
) -> Result<(), ServerError> {
    let msg_sender_addr = Address::from_str(msg_sender)
        .map_err(|_| ServerError::InvalidAddress(msg_sender.to_owned()))?;

    if !ingame_wallet_manager.is_ingame_wallet_attached(&msg_sender_addr) {
        return Err(ServerError::InvalidIngameWallet(
            msg_sender_addr.to_string(),
        ));
    }

    let stake = U256::from_dec_str(STAKE).expect("Invalid STAKE value in config.rs!");

    //#NOTE: further spec needed
    let default_balance = &U256::from(0);
    let balance = balance_manager
        .get_balance(&msg_sender_addr)
        .unwrap_or(default_balance);

    if balance < &stake {
        return Err(ServerError::InsufficientTournamentFee(
            balance.to_string(),
            stake.to_string(),
        ));
    }

    Ok(())
}

pub async fn advance_state(
    request: AdvanceRequest,
    room_manager: &mut RoomManager,
    balance_manager: &mut BalanceManager,
    ingame_wallet_manager: &mut IngameWalletManager,
    http_dispatcher_url: &str,
    ether_portal: &str,
    dapp_address: &str,
) -> Result<FinishStatus, DazzleError> {
    log::debug!("advance_state");

    if request.metadata.is_none() {
        log::debug!("No metadata inside the payload");
        return Ok(FinishStatus::Reject);
    }

    let metadata = request.metadata.unwrap();
    let msg_sender = metadata.msg_sender.clone();
    log::debug!("advance_state.msg_sender: {}", msg_sender);

    let hex_payload = request.payload.trim_start_matches("0x");
    log::debug!("hex_payload: {}", &hex_payload);

    if msg_sender.to_lowercase() == ether_portal.to_lowercase() {
        log::debug!("handle_deposit");

        let padding_payload = &format!("{:0>128}", hex_payload);
        let bz_payload = hex::decode(padding_payload).map_err(|_| ServerError::InvalidHex)?;
        return handle_deposit(http_dispatcher_url, balance_manager, &bz_payload).await;
    }

    let bz_payload = hex::decode(hex_payload).map_err(|e| {
        log::error!("Failed to decode hex payload: {}", e.to_string());
        ServerError::InvalidHex
    })?;

    let game_req: GameRequest =
        serde_json::from_slice(&bz_payload).map_err(|_| ServerError::InvalidRequest)?;

    let game_operation: Result<DazzleOperation, strum::ParseError> = game_req.operation.parse();
    let vec_request = general_purpose::STANDARD
        .decode(&game_req.data)
        .map_err(|e| {
            log::debug!(
                "Failed to decode base64 payload: {} - {}",
                game_req.data.clone(),
                e.to_string()
            );
            ServerError::InvalidRequest
        })?;

    let new_rng_seed = metadata.timestamp + metadata.input_index;

    match game_operation {
        Ok(DazzleOperation::CreatePrivateRoom) => {
            if let Err(e) = auth_msg_sender(balance_manager, ingame_wallet_manager, &msg_sender) {
                log::error!("Report Error: {}", &e);
                return send_report(http_dispatcher_url, &serialize_error_report(e.into())).await;
            }

            match create_private_room(room_manager, http_dispatcher_url, &vec_request).await {
                Ok(state) => Ok(state),
                Err(e) => {
                    log::error!("Report Error: {}", &e);
                    send_report(http_dispatcher_url, &serialize_error_report(e)).await
                }
            }
        }
        Ok(DazzleOperation::JoinPrivateRoom) => {
            if let Err(e) = auth_msg_sender(balance_manager, ingame_wallet_manager, &msg_sender) {
                log::error!("Report Error: {}", &e);
                return send_report(http_dispatcher_url, &serialize_error_report(e.into())).await;
            }

            match join_private_room(
                room_manager,
                http_dispatcher_url,
                &vec_request,
                new_rng_seed,
            )
            .await
            {
                Ok(state) => Ok(state),
                Err(e) => {
                    log::error!("Report Error: {}", &e);
                    send_report(http_dispatcher_url, &serialize_error_report(e)).await
                }
            }
        }
        Ok(DazzleOperation::CancelRoom) => {
            if let Err(e) = auth_msg_sender(balance_manager, ingame_wallet_manager, &msg_sender) {
                log::error!("Report Error: {}", &e);
                return send_report(http_dispatcher_url, &serialize_error_report(e.into())).await;
            }

            match cancel_room(room_manager, http_dispatcher_url, &vec_request).await {
                Ok(state) => Ok(state),
                Err(e) => {
                    log::error!("Report Error: {}", &e);
                    send_report(http_dispatcher_url, &serialize_error_report(e)).await
                }
            }
        }

        Ok(DazzleOperation::GameOver) => {
            if let Err(e) = auth_msg_sender(balance_manager, ingame_wallet_manager, &msg_sender) {
                log::error!("Report Error: {}", &e);
                return send_report(http_dispatcher_url, &serialize_error_report(e.into())).await;
            }

            match game_over(
                room_manager,
                balance_manager,
                http_dispatcher_url,
                &vec_request,
            )
            .await
            {
                Ok(state) => Ok(state),
                Err(e) => {
                    log::error!("Report Error: {}", &e);
                    send_report(http_dispatcher_url, &serialize_error_report(e)).await
                }
            }
        }
        Ok(DazzleOperation::Move) => {
            if let Err(e) = auth_msg_sender(balance_manager, ingame_wallet_manager, &msg_sender) {
                log::error!("Report Error: {}", &e);
                return send_report(http_dispatcher_url, &serialize_error_report(e.into())).await;
            }

            match action_move(
                room_manager,
                http_dispatcher_url,
                &vec_request,
                new_rng_seed,
            )
            .await
            {
                Ok(state) => Ok(state),
                Err(e) => {
                    log::error!("Report Error: {}", &e);
                    send_report(http_dispatcher_url, &serialize_error_report(e)).await
                }
            }
        }
        Ok(DazzleOperation::ActivateSkill) => {
            if let Err(e) = auth_msg_sender(balance_manager, ingame_wallet_manager, &msg_sender) {
                log::error!("Report Error: {}", &e);
                return send_report(http_dispatcher_url, &serialize_error_report(e.into())).await;
            }

            match activate_skill(
                room_manager,
                http_dispatcher_url,
                &vec_request,
                new_rng_seed,
            )
            .await
            {
                Ok(state) => Ok(state),
                Err(e) => {
                    log::error!("Report Error: {}", &e);
                    send_report(http_dispatcher_url, &serialize_error_report(e)).await
                }
            }
        }
        Ok(DazzleOperation::QuitGame) => {
            if let Err(e) = auth_msg_sender(balance_manager, ingame_wallet_manager, &msg_sender) {
                log::error!("Report Error: {}", &e);
                return send_report(http_dispatcher_url, &serialize_error_report(e.into())).await;
            }

            match quit_game(room_manager, http_dispatcher_url, &vec_request).await {
                Ok(state) => Ok(state),
                Err(e) => {
                    log::error!("Report Error: {}", &e);
                    send_report(http_dispatcher_url, &serialize_error_report(e)).await
                }
            }
        }
        Ok(DazzleOperation::AttachIngameWallet) => {
            match attach_ingame_wallet(
                ingame_wallet_manager,
                http_dispatcher_url,
                metadata,
                &vec_request,
            )
            .await
            {
                Ok(state) => Ok(state),
                Err(e) => {
                    log::error!("Report Error: {}", &e);
                    send_report(http_dispatcher_url, &serialize_error_report(e)).await
                }
            }
        }
        Ok(DazzleOperation::TransferBalance) => {
            match transfer(
                balance_manager,
                http_dispatcher_url,
                dapp_address,
                metadata,
                &vec_request,
            )
            .await
            {
                Ok(state) => Ok(state),
                Err(e) => {
                    log::error!("Report Error: {}", &e);
                    send_report(http_dispatcher_url, &serialize_error_report(e)).await
                }
            }
        }
        Err(_) => {
            log::debug!("Not supported action");
            Ok(FinishStatus::Reject)
        }
    }
}

fn serialize_error_report(err: DazzleError) -> String {
    let dazzle_report = DazzleReport {
        error_message: err.to_string(),
    };
    serde_json::to_string(&dazzle_report).unwrap()
}

pub async fn rollup() {
    let http_dispatcher_url =
        env::var("ROLLUP_HTTP_SERVER_URL").unwrap_or(String::from("http://127.0.0.1:5004"));

    //#NOTE: These address can be known beforehand by using `sunodo address-book` command, and is suggested to hardcoded in dapp
    let address_relay_contract = env::var("DAPP_ADDRESS_RELAY_CONTRACT")
        .unwrap_or(String::from("0xF5DE34d6BbC0446E2a45719E718efEbaaE179daE"));

    let ether_portal_contract = env::var("DAPP_ETHER_PORTAL_CONTRACT")
        .unwrap_or(String::from("0xFfdbe43d4c855BF7e0f105c400A50857f53AB044"));

    log::debug!("rollup_server url is : {}", http_dispatcher_url);
    log::debug!("Sending finish");

    let mut room_manager = RoomManager::new();
    let mut balance_manager = BalanceManager::new();
    let mut ingame_wallet_manager = IngameWalletManager::new();
    let mut status = FinishStatus::Accept;
    let mut dapp_address = env::var("DAZZLE_DAPP_CONTRACT").unwrap_or(String::from(""));
    log::debug!("Init dapp address: {}", dapp_address.clone());

    loop {
        let resp = match send_finish_request(&http_dispatcher_url, status.clone()).await {
            Some(resp) => resp,
            None => {
                continue;
            }
        };

        if resp.status() == StatusCode::ACCEPTED {
            log::debug!("No pending rollup request, trying again");
        } else {
            let buf = match hyper::body::to_bytes(resp).await {
                Ok(bz) => bz.to_vec(),
                Err(e) => {
                    log::debug!("Failed to handle /finish response: {}", e);
                    continue;
                }
            };

            let rollup = match serde_json::from_slice::<RollupResponse>(&buf) {
                Ok(json) => json,
                Err(e) => {
                    log::debug!("Failed to deserialize RollupResponse: {}", e);
                    continue;
                }
            };

            if let Some(metadata) = rollup.data.metadata.clone() {
                if metadata.msg_sender.to_lowercase() == address_relay_contract.to_lowercase() {
                    dapp_address = rollup.data.payload;
                    log::debug!("Captured dapp address: {}", dapp_address);
                    continue;
                }
            }

            let rollup_req: Result<RequestType, strum::ParseError> = rollup.request_type.parse();

            match rollup_req {
                Ok(RequestType::AdvanceState) => {
                    //Now we need to handle error case so that we can send reject status through /finish call
                    status = advance_state(
                        rollup.data,
                        &mut room_manager,
                        &mut balance_manager,
                        &mut ingame_wallet_manager,
                        &http_dispatcher_url,
                        &ether_portal_contract,
                        &dapp_address,
                    )
                    .await
                    .unwrap_or_else(|e| {
                        log::error!("Error occurred in advance_state: {}", e);
                        FinishStatus::Reject
                    });
                }
                Ok(RequestType::InspectState) => {
                    status = inspect_state(
                        &room_manager,
                        &balance_manager,
                        &ingame_wallet_manager,
                        &http_dispatcher_url,
                    )
                    .await
                    .unwrap_or_else(|e| {
                        log::error!("Error occurred in inspect_state: {}", e);
                        FinishStatus::Reject
                    });
                }
                Err(e) => {
                    log::error!("Error occurred while handling rollup request: {}", e);
                }
            }
        }
    }
}
