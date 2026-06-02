use super::*;

pub(super) fn handle_peer_command(
    daemon: &RpcDaemon,
    path_hash: [u8; 16],
    data: Option<rmpv::Value>,
    error_invalid_data: u8,
    error_not_found: u8,
) -> Option<ControlResponse> {
    let method = if path_hash == control_path_hash("/pn/peer/sync") {
        "peer_sync"
    } else if path_hash == control_path_hash("/pn/peer/unpeer") {
        "peer_unpeer"
    } else {
        return None;
    };
    let Some((peer_hex, transfer_limit_kb)) = peer_request_from_data(data) else {
        return Some(ControlResponse::Code(error_invalid_data));
    };
    if !peer_exists(daemon, peer_hex.as_str()) {
        return Some(ControlResponse::Code(error_not_found));
    }
    let mut params = json!({ "peer": peer_hex });
    if let Some(transfer_limit_kb) = transfer_limit_kb {
        params["transfer_limit_kb"] = json!(transfer_limit_kb);
    }
    let _ =
        daemon.handle_rpc(RpcRequest { id: 0, method: method.to_string(), params: Some(params) });
    Some(ControlResponse::Bool(true))
}

fn peer_request_from_data(data: Option<rmpv::Value>) -> Option<(String, Option<f64>)> {
    match data {
        Some(rmpv::Value::Binary(bytes)) if bytes.len() == 16 => Some((hex::encode(bytes), None)),
        Some(rmpv::Value::Array(entries)) => {
            let peer = match entries.first()? {
                rmpv::Value::Binary(bytes) if bytes.len() == 16 => hex::encode(bytes),
                _ => return None,
            };
            let transfer_limit_kb = entries.get(1).and_then(transfer_limit_kb_from_value);
            Some((peer, transfer_limit_kb))
        }
        _ => None,
    }
}

fn transfer_limit_kb_from_value(value: &rmpv::Value) -> Option<f64> {
    let limit = match value {
        rmpv::Value::F64(value) => Some(*value),
        rmpv::Value::F32(value) => Some((*value).into()),
        rmpv::Value::Integer(value) => value.as_f64(),
        rmpv::Value::String(value) => value.as_str()?.trim().parse::<f64>().ok(),
        rmpv::Value::Binary(value) => std::str::from_utf8(value).ok()?.trim().parse::<f64>().ok(),
        rmpv::Value::Boolean(value) => Some(f64::from(*value as u8)),
        _ => None,
    }?;
    limit.is_finite().then_some(limit.max(0.0))
}

fn peer_exists(daemon: &RpcDaemon, peer_hex: &str) -> bool {
    daemon
        .handle_rpc(RpcRequest { id: 0, method: "list_peers".to_string(), params: None })
        .ok()
        .and_then(|response| response.result)
        .and_then(|value| value.get("peers").cloned())
        .and_then(|value| value.as_array().cloned())
        .map(|rows| {
            rows.iter().any(|row| row.get("peer").and_then(Value::as_str) == Some(peer_hex))
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ERROR_INVALID_DATA: u8 = 0xF4;
    const ERROR_NOT_FOUND: u8 = 0xFD;

    #[test]
    fn peer_command_returns_none_for_unhandled_path() {
        let daemon = RpcDaemon::test_instance();

        let response = handle_peer_command(
            &daemon,
            control_path_hash("/pn/get/stats"),
            Some(rmpv::Value::Binary(vec![0; 16])),
            ERROR_INVALID_DATA,
            ERROR_NOT_FOUND,
        );

        assert!(response.is_none());
    }

    #[test]
    fn peer_command_returns_not_found_for_unknown_peer() {
        let daemon = RpcDaemon::test_instance();

        let response = handle_peer_command(
            &daemon,
            control_path_hash("/pn/peer/sync"),
            Some(rmpv::Value::Binary(vec![0xA5; 16])),
            ERROR_INVALID_DATA,
            ERROR_NOT_FOUND,
        );

        assert!(matches!(response, Some(ControlResponse::Code(ERROR_NOT_FOUND))));
    }

    #[test]
    fn peer_request_accepts_transfer_limit_array_payload() {
        let peer_bytes = [0xA5; 16];

        let (peer_hex, transfer_limit_kb) = peer_request_from_data(Some(rmpv::Value::Array(vec![
            rmpv::Value::Binary(peer_bytes.to_vec()),
            rmpv::Value::F64(42.5),
        ])))
        .expect("peer request");

        assert_eq!(peer_hex, hex::encode(peer_bytes));
        assert_eq!(transfer_limit_kb, Some(42.5));
    }

    #[test]
    fn peer_unpeer_command_delegates_to_daemon_rpc() {
        let daemon = RpcDaemon::test_instance();
        let peer_bytes = [0xB6; 16];
        let peer_hex = hex::encode(peer_bytes);
        daemon
            .handle_rpc(RpcRequest {
                id: 1,
                method: "peer_sync".to_string(),
                params: Some(json!({ "peer": peer_hex })),
            })
            .expect("seed peer");

        let response = handle_peer_command(
            &daemon,
            control_path_hash("/pn/peer/unpeer"),
            Some(rmpv::Value::Binary(peer_bytes.to_vec())),
            ERROR_INVALID_DATA,
            ERROR_NOT_FOUND,
        );

        assert!(matches!(response, Some(ControlResponse::Bool(true))));
        let peers = daemon
            .handle_rpc(RpcRequest { id: 2, method: "list_peers".to_string(), params: None })
            .expect("list peers")
            .result
            .expect("peers result");
        assert_eq!(peers["peers"].as_array().map(Vec::len), Some(0));
    }
}
