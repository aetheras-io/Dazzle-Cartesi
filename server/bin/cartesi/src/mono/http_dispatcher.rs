use base64::{engine::general_purpose, Engine as _};
use domain::cartesi::{
    DinderNotice, FinishStatus, IndexResponse, Notice, NoticeType, Report, Voucher,
};
use domain::game_core::game::Room;
use domain::game_core::{DinderError, ServerError};
use ethers_core::utils::hex;
use hyper::{header as HyperHeader, Body, Client, Method, Request, Response};

pub async fn send_room_snapshot_notice(
    http_dispatcher_url: &str,
    user: &str,
    room: &Room,
    balance: Option<String>,
) -> Result<FinishStatus, DinderError> {
    let snapshot_room = room.snapshot();
    let room_notice = serde_json::to_string(&snapshot_room).unwrap();

    send_notice(
        http_dispatcher_url,
        NoticeType::Room,
        &room_notice,
        user,
        balance,
    )
    .await
}

pub async fn send_notice(
    http_dispatcher_url: &str,
    notice_type: NoticeType,
    payload: &str,
    user: &str,
    balance: Option<String>,
) -> Result<FinishStatus, DinderError> {
    log::debug!("Call to Http Dispatcher: Adding Notice");
    let client = Client::new();

    let base64_payload = general_purpose::STANDARD.encode(payload);
    log::debug!("Base64-encoded payload: {}", base64_payload);

    let inner_notice = DinderNotice {
        notice_type,
        base64_content: base64_payload,
        user: user.to_owned(),
        balance,
    };

    let inner_json = serde_json::to_string(&inner_notice).unwrap();
    let hexed_inner_notice = hex::encode(inner_json);

    let notice = Notice {
        payload: format!("0x{}", hexed_inner_notice),
    };

    let notice_json = serde_json::to_string(&notice).unwrap();
    log::debug!("notice_json: {}", notice_json);

    let notice_req = Request::builder()
        .method(Method::POST)
        .header(HyperHeader::CONTENT_TYPE, "application/json")
        .uri(format!("{}/notice", http_dispatcher_url))
        .body(Body::from(notice_json))
        .map_err(|_| ServerError::FailedToBuildRequest)?;

    let notice_resp = client
        .request(notice_req)
        .await
        .map_err(|_| ServerError::FailedToSendNotice)?;

    let notice_status = notice_resp.status();
    let bz = hyper::body::to_bytes(notice_resp)
        .await
        .map_err(|_| ServerError::FailedToHandleResponse)?;

    let id_response = serde_json::from_slice::<IndexResponse>(&bz)
        .map_err(|_| ServerError::FailedToHandleResponse)?;

    log::debug!(
        "Received notice status {} body {:?}",
        notice_status,
        &id_response
    );

    Ok(FinishStatus::Accept)
}

pub async fn send_finish_request(
    http_dispatcher_url: &str,
    status: FinishStatus,
) -> Option<Response<Body>> {
    log::debug!("Call to Http Dispatcher: Finishing");
    let client = Client::new();

    let status_value = status.to_string();
    log::debug!("status_value: {}", status_value);

    let mut json_status = std::collections::HashMap::new();
    json_status.insert("status", status_value);

    let finish_req = match Request::builder()
        .method(Method::POST)
        .header(HyperHeader::CONTENT_TYPE, "application/json")
        .uri(format!("{}/finish", http_dispatcher_url))
        .body(Body::from(serde_json::to_string(&json_status).unwrap()))
    {
        Ok(req) => req,
        Err(e) => {
            log::debug!("error while generating send_finish_request body: {}", e);
            return None;
        }
    };

    match client.request(finish_req).await {
        Ok(resp) => Some(resp),
        Err(e) => {
            log::debug!("error while send_finish_request: {}", e);
            return None;
        }
    }
}

pub async fn send_report(
    http_dispatcher_url: &str,
    payload: &str,
) -> Result<FinishStatus, DinderError> {
    log::debug!("Call to Http Dispatcher: Adding Report");
    let client = Client::new();

    let hexed_payload = hex::encode(payload);

    let report = Report {
        payload: format!("0x{}", hexed_payload),
    };

    let report_json = serde_json::to_string(&report).unwrap();

    let report_req = Request::builder()
        .method(Method::POST)
        .header(HyperHeader::CONTENT_TYPE, "application/json")
        .uri(format!("{}/report", http_dispatcher_url))
        .body(Body::from(report_json))
        .map_err(|_| ServerError::FailedToBuildRequest)?;

    let report_resp = client
        .request(report_req)
        .await
        .map_err(|_| ServerError::FailedToSendReport)?;

    let report_status = report_resp.status();
    let bz = hyper::body::to_bytes(report_resp)
        .await
        .map_err(|_| ServerError::FailedToHandleResponse)?
        .to_vec();

    let resp_string = std::str::from_utf8(&bz).map_err(|_| ServerError::FailedToHandleResponse)?;

    log::debug!(
        "Received report status {} body {:?}",
        report_status,
        resp_string
    );

    Ok(FinishStatus::Accept)
}

#[allow(dead_code)]
pub async fn send_voucher(
    http_dispatcher_url: &str,
    dapp_address: &str,
    payload: &[u8],
) -> Result<FinishStatus, DinderError> {
    log::debug!("Call to Http Dispatcher: Adding Voucher");
    let client = Client::new();
    let hexed_payload = hex::encode(payload);
    log::debug!("Hex-encoded payload: {}", hexed_payload);

    let voucher = Voucher {
        destination: dapp_address.to_owned(),
        payload: format!("0x{}", hexed_payload),
    };

    let voucher_json = serde_json::to_string(&voucher).unwrap();
    log::debug!("voucher_json: {}", voucher_json);

    let voucher_req = Request::builder()
        .method(Method::POST)
        .header(HyperHeader::CONTENT_TYPE, "application/json")
        .uri(format!("{}/voucher", http_dispatcher_url))
        .body(Body::from(voucher_json))
        .map_err(|_| ServerError::FailedToBuildRequest)?;

    let voucher_resp = client
        .request(voucher_req)
        .await
        .map_err(|_| ServerError::FailedToSendReport)?;

    let voucher_status = voucher_resp.status();
    let bz = hyper::body::to_bytes(voucher_resp)
        .await
        .map_err(|_| ServerError::FailedToHandleResponse)?
        .to_vec();

    let resp_string = std::str::from_utf8(&bz).map_err(|_| ServerError::FailedToHandleResponse)?;

    log::debug!(
        "Received voucher status {} body {:?}",
        voucher_status,
        resp_string
    );

    Ok(FinishStatus::Accept)
}
