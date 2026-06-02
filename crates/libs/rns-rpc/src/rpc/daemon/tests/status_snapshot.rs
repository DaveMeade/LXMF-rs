#[test]
fn daemon_status_ex_reads_cached_status_snapshot() {
    let daemon = RpcDaemon::test_instance();
    daemon.replace_interfaces(vec![InterfaceRecord {
        kind: "tcp_client".to_string(),
        enabled: true,
        host: Some("rmap.world".to_string()),
        port: Some(4242),
        name: Some("primary".to_string()),
        settings: None,
    }]);
    daemon.accept_announce("peer-1".to_string(), 1_700_000_000).expect("announce");

    let delivery = daemon
        .handle_rpc(rpc_request(
            10,
            "set_delivery_policy",
            json!({
                "auth_required": true,
                "allowed_destinations": ["alpha"],
                "denied_destinations": ["beta"],
                "ignored_destinations": ["gamma"],
                "prioritised_destinations": ["delta"],
            }),
        ))
        .expect("set delivery policy");
    assert!(delivery.error.is_none());

    let propagation = daemon
        .handle_rpc(rpc_request(
            11,
            "propagation_enable",
            json!({
                "enabled": true,
                "store_root": "/tmp/propagation",
                "target_cost": 9,
                "stamp_cost_flexibility": 4,
            }),
        ))
        .expect("enable propagation");
    assert!(propagation.error.is_none());

    let stamp = daemon
        .handle_rpc(rpc_request(
            12,
            "stamp_policy_set",
            json!({
                "target_cost": 11,
                "flexibility": 3,
            }),
        ))
        .expect("set stamp policy");
    assert!(stamp.error.is_none());

    let response = daemon
        .handle_rpc(RpcRequest { id: 13, method: "daemon_status_ex".to_string(), params: None })
        .expect("daemon status");
    let result = response.result.expect("daemon status result");

    assert_eq!(result["peer_count"].as_u64(), Some(1));
    assert_eq!(result["interface_count"].as_u64(), Some(1));
    assert_eq!(result["interfaces"][0]["name"].as_str(), Some("primary"));
    assert_eq!(result["delivery_policy"]["auth_required"].as_bool(), Some(true));
    assert_eq!(result["delivery_policy"]["allowed_destinations"][0].as_str(), Some("alpha"));
    assert_eq!(result["propagation"]["enabled"].as_bool(), Some(true));
    assert_eq!(result["propagation"]["target_cost"].as_u64(), Some(9));
    assert_eq!(result["propagation"]["stamp_cost_flexibility"].as_u64(), Some(4));
    assert_eq!(result["stamp_policy"]["target_cost"].as_u64(), Some(11));
    assert_eq!(result["stamp_policy"]["flexibility"].as_u64(), Some(3));
    assert_eq!(result["stamp_policy"]["enforce"].as_bool(), Some(true));
}

#[test]
fn propagation_policy_is_reported_and_enforced_for_new_peers() {
    let daemon = RpcDaemon::test_instance();

    let propagation = daemon
        .handle_rpc(rpc_request(
            20,
            "propagation_enable",
            json!({
                "enabled": true,
                "target_cost": 9,
                "stamp_cost_flexibility": 5,
                "delivery_limit": 321,
                "propagation_limit": 654,
                "sync_limit": 987,
                "static_peers": ["static-peer"],
                "max_peers": 1,
                "from_static_only": true,
                "peering_cost": 18,
                "remote_peering_cost_max": 26,
            }),
        ))
        .expect("enable propagation");
    assert!(propagation.error.is_none());

    let result = daemon
        .handle_rpc(RpcRequest { id: 21, method: "daemon_status_ex".to_string(), params: None })
        .expect("daemon status")
        .result
        .expect("daemon status result");
    assert_eq!(result["propagation"]["static_peers"][0].as_str(), Some("static-peer"));
    assert_eq!(result["propagation"]["stamp_cost_flexibility"].as_u64(), Some(5));
    assert_eq!(result["propagation"]["delivery_limit"].as_u64(), Some(321));
    assert_eq!(result["propagation"]["propagation_limit"].as_u64(), Some(654));
    assert_eq!(result["propagation"]["sync_limit"].as_u64(), Some(987));
    assert_eq!(result["propagation"]["max_peers"].as_u64(), Some(1));
    assert_eq!(result["propagation"]["from_static_only"].as_bool(), Some(true));
    assert_eq!(result["propagation"]["peering_cost"].as_u64(), Some(18));
    assert_eq!(result["propagation"]["remote_peering_cost_max"].as_u64(), Some(26));
    assert_eq!(result["propagation"]["message_storage_limit_mb"].as_u64(), None);

    daemon.accept_announce("static-peer".to_string(), 1_700_000_000).expect("static peer accepted");
    daemon
        .accept_announce("dynamic-peer".to_string(), 1_700_000_001)
        .expect("dynamic announce accepted");
    let peers = daemon
        .handle_rpc(RpcRequest { id: 22, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let rows = peers["peers"].as_array().expect("peer rows");
    assert_eq!(rows.len(), 1, "non-static announce should not become a peered node");
    assert_eq!(rows[0]["peer"].as_str(), Some("static-peer"));
}

#[test]
fn propagation_enable_activates_static_peers_like_python() {
    let daemon = RpcDaemon::test_instance();

    let response = daemon
        .handle_rpc(rpc_request(
            23,
            "propagation_enable",
            json!({
                "enabled": true,
                "static_peers": ["peer-static"],
            }),
        ))
        .expect("enable propagation");
    assert!(response.error.is_none());

    let status = daemon
        .handle_rpc(RpcRequest { id: 24, method: "daemon_status_ex".to_string(), params: None })
        .expect("daemon status")
        .result
        .expect("daemon status result");
    assert_eq!(status["peer_count"].as_u64(), Some(1));

    let peers = daemon
        .handle_rpc(RpcRequest { id: 25, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let rows = peers["peers"].as_array().expect("peer rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["peer"].as_str(), Some("peer-static"));
    assert_eq!(rows[0]["peer_type"].as_str(), Some("static"));
    assert_eq!(rows[0]["type"].as_str(), Some("static"));
    assert_eq!(rows[0]["alive"].as_bool(), Some(false));
    assert_eq!(rows[0]["last_seen"].as_i64(), Some(0));
}

#[test]
fn message_storage_stats_track_count_and_bytes() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            30,
            "propagation_enable",
            json!({
                "enabled": true,
                "message_storage_limit_mb": 4,
            }),
        ))
        .expect("enable propagation");

    daemon
        .accept_inbound(MessageRecord {
            id: "msg-1".to_string(),
            source: "src".to_string(),
            destination: "dst".to_string(),
            title: "hello".to_string(),
            content: "world".to_string(),
            timestamp: 1_700_000_000,
            direction: "in".to_string(),
            fields: Some(json!({"k":"v"})),
            receipt_status: None,
        })
        .expect("store inbound");

    let (count, bytes) = daemon.message_storage_stats().expect("storage stats");
    assert_eq!(count, 1);
    assert!(bytes > 0);

    let result = daemon
        .handle_rpc(RpcRequest { id: 31, method: "daemon_status_ex".to_string(), params: None })
        .expect("daemon status")
        .result
        .expect("daemon status result");
    assert_eq!(result["message_count"].as_u64(), Some(1));
    assert_eq!(result["propagation"]["message_storage_limit_mb"].as_u64(), Some(4));
}

#[test]
fn propagation_message_storage_zero_limit_disables_limit_like_python() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            32,
            "propagation_enable",
            json!({
                "enabled": true,
                "message_storage_limit_mb": 4,
            }),
        ))
        .expect("enable propagation");
    daemon
        .handle_rpc(rpc_request(
            33,
            "propagation_enable",
            json!({
                "enabled": true,
                "message_storage_limit_mb": 0,
            }),
        ))
        .expect("clear propagation storage limit");

    let result = daemon
        .handle_rpc(RpcRequest { id: 34, method: "daemon_status_ex".to_string(), params: None })
        .expect("daemon status")
        .result
        .expect("daemon status result");
    assert_eq!(result["propagation"]["message_storage_limit_mb"], JsonValue::Null);
}

#[test]
fn duplicate_inbound_message_does_not_replace_existing_record_like_python() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .accept_inbound(MessageRecord {
            id: "duplicate-inbound".to_string(),
            source: "src-a".to_string(),
            destination: "dst".to_string(),
            title: "original title".to_string(),
            content: "original content".to_string(),
            timestamp: 1_700_000_000,
            direction: "in".to_string(),
            fields: Some(json!({"version": 1})),
            receipt_status: None,
        })
        .expect("store original inbound");
    daemon
        .accept_inbound(MessageRecord {
            id: "duplicate-inbound".to_string(),
            source: "src-b".to_string(),
            destination: "dst".to_string(),
            title: "replacement title".to_string(),
            content: "replacement content".to_string(),
            timestamp: 1_700_000_001,
            direction: "in".to_string(),
            fields: Some(json!({"version": 2})),
            receipt_status: None,
        })
        .expect("ignore duplicate inbound");

    let result = daemon
        .handle_rpc(RpcRequest { id: 35, method: "list_messages".to_string(), params: None })
        .expect("list messages")
        .result
        .expect("list messages result");
    let messages = result["messages"].as_array().expect("messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["source"].as_str(), Some("src-a"));
    assert_eq!(messages[0]["title"].as_str(), Some("original title"));
    assert_eq!(messages[0]["content"].as_str(), Some("original content"));
    assert_eq!(messages[0]["fields"]["version"].as_u64(), Some(1));
}

#[test]
fn list_messages_cursor_paginates_same_second_records_by_id() {
    let daemon = RpcDaemon::test_instance();
    for id in ["msg-a", "msg-c", "msg-b"] {
        daemon
            .accept_inbound(MessageRecord {
                id: id.to_string(),
                source: "src".to_string(),
                destination: "dst".to_string(),
                title: id.to_string(),
                content: String::new(),
                timestamp: 1_700_000_100,
                direction: "in".to_string(),
                fields: None,
                receipt_status: None,
            })
            .expect("store same-second message");
    }

    let first = daemon
        .handle_rpc(rpc_request(36, "list_messages", json!({ "limit": 2 })))
        .expect("list first page")
        .result
        .expect("first page result");
    let first_messages = first["messages"].as_array().expect("first messages");
    assert_eq!(
        first_messages.iter().map(|row| row["id"].as_str().unwrap()).collect::<Vec<_>>(),
        vec!["msg-c", "msg-b"]
    );
    assert_eq!(first["next_cursor"].as_str(), Some("1700000100:msg-b"));

    let second = daemon
        .handle_rpc(rpc_request(
            37,
            "list_messages",
            json!({ "cursor": first["next_cursor"].as_str().unwrap(), "limit": 2 }),
        ))
        .expect("list second page")
        .result
        .expect("second page result");
    let second_messages = second["messages"].as_array().expect("second messages");
    assert_eq!(
        second_messages.iter().map(|row| row["id"].as_str().unwrap()).collect::<Vec<_>>(),
        vec!["msg-a"]
    );
    assert_eq!(second["next_cursor"], JsonValue::Null);
}

#[test]
fn list_messages_omits_next_cursor_when_exact_limit_is_exhausted() {
    let daemon = RpcDaemon::test_instance();
    for id in ["msg-a", "msg-b"] {
        daemon
            .accept_inbound(MessageRecord {
                id: id.to_string(),
                source: "src".to_string(),
                destination: "dst".to_string(),
                title: id.to_string(),
                content: String::new(),
                timestamp: 1_700_000_101,
                direction: "in".to_string(),
                fields: None,
                receipt_status: None,
            })
            .expect("store exact-limit message");
    }

    let result = daemon
        .handle_rpc(rpc_request(38, "list_messages", json!({ "limit": 2 })))
        .expect("list exact page")
        .result
        .expect("exact page result");

    assert_eq!(result["messages"].as_array().map(Vec::len), Some(2));
    assert_eq!(result["next_cursor"], JsonValue::Null);
}

#[test]
fn autopeer_disabled_keeps_announced_peer_unpeered() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            40,
            "propagation_enable",
            json!({
                "enabled": true,
                "autopeer": false,
                "autopeer_maxdepth": 2,
            }),
        ))
        .expect("enable propagation");

    daemon
        .accept_announce_with_metadata(
            "peer-auto".to_string(),
            1_700_000_010,
            Some("Peer Auto".to_string()),
            Some("announce".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(1),
            None,
            None,
            None,
            None,
        )
        .expect("accept announce");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 41, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    assert_eq!(peers["peers"].as_array().map(|rows| rows.len()), Some(0));

    let status = daemon
        .handle_rpc(RpcRequest { id: 42, method: "daemon_status_ex".to_string(), params: None })
        .expect("daemon status")
        .result
        .expect("daemon status result");
    assert_eq!(status["propagation"]["autopeer"].as_bool(), Some(false));
    assert_eq!(status["propagation"]["autopeer_maxdepth"].as_u64(), Some(2));
}

#[test]
fn announce_received_persists_stamp_cost_in_announce_log() {
    let daemon = RpcDaemon::test_instance();
    let announce = daemon
        .handle_rpc(rpc_request(
            43,
            "announce_received",
            json!({
                "peer": "peer-stamp",
                "timestamp": 1_700_000_011i64,
                "stamp_cost": 21,
                "stamp_cost_flexibility": 4,
            }),
        ))
        .expect("announce received");
    assert!(announce.error.is_none());

    let result = daemon
        .handle_rpc(RpcRequest { id: 44, method: "list_announces".to_string(), params: None })
        .expect("list announces")
        .result
        .expect("list announces result");
    let rows = result["announces"].as_array().expect("announce rows");
    let row = rows.first().expect("announce row");
    assert_eq!(row["peer"].as_str(), Some("peer-stamp"));
    assert_eq!(row["timestamp"].as_i64(), Some(1_700_000_011));
    assert_eq!(row["stamp_cost"].as_u64(), Some(21));
    assert_eq!(row["stamp_cost_flexibility"].as_u64(), Some(4));
}

#[test]
fn list_announces_omits_next_cursor_when_exact_limit_is_exhausted() {
    let daemon = RpcDaemon::test_instance();
    for peer in ["peer-b", "peer-a"] {
        daemon
            .handle_rpc(rpc_request(
                45,
                "announce_received",
                json!({
                    "peer": peer,
                    "timestamp": 1_700_000_015i64,
                    "aspect": "lxmf.delivery",
                }),
            ))
            .expect("announce received");
    }

    let result = daemon
        .handle_rpc(rpc_request(46, "list_announces", json!({ "limit": 2 })))
        .expect("list exact announces")
        .result
        .expect("exact announces result");

    assert_eq!(result["announces"].as_array().map(Vec::len), Some(2));
    assert_eq!(result["next_cursor"], JsonValue::Null);
}

#[test]
fn announce_received_parses_delivery_stamp_cost_from_python_app_data() {
    let daemon = RpcDaemon::test_instance();
    let app_data = rmp_serde::to_vec_named(&MsgPackValue::Array(vec![
        MsgPackValue::Binary(b"Peer Stamp".to_vec()),
        MsgPackValue::from(22),
    ]))
    .expect("encode app data");

    let announce = daemon
        .handle_rpc(rpc_request(
            45,
            "announce_received",
            json!({
                "peer": "peer-delivery-stamp",
                "timestamp": 1_700_000_012i64,
                "app_data_hex": hex::encode(app_data),
                "aspect": "lxmf.delivery",
            }),
        ))
        .expect("announce received");
    assert!(announce.error.is_none());

    assert_eq!(
        daemon.outbound_stamp_cost_for("peer-delivery-stamp").expect("stamp cost lookup"),
        Some(22)
    );
}

#[test]
fn announce_received_ignores_python_invalid_delivery_stamp_cost_from_app_data() {
    let daemon = RpcDaemon::test_instance();
    let app_data = rmp_serde::to_vec_named(&MsgPackValue::Array(vec![
        MsgPackValue::Binary(b"Peer Stamp".to_vec()),
        MsgPackValue::from(255),
    ]))
    .expect("encode app data");

    let announce = daemon
        .handle_rpc(rpc_request(
            46,
            "announce_received",
            json!({
                "peer": "peer-invalid-delivery-stamp",
                "timestamp": 1_700_000_012i64,
                "app_data_hex": hex::encode(app_data),
                "aspect": "lxmf.delivery",
            }),
        ))
        .expect("announce received");
    assert!(announce.error.is_none());

    assert_eq!(
        daemon.outbound_stamp_cost_for("peer-invalid-delivery-stamp").expect("stamp cost lookup"),
        None
    );
}

#[test]
fn announce_received_parses_propagation_peer_limits_from_python_app_data() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            46,
            "propagation_enable",
            json!({
                "enabled": true,
                "autopeer": true,
            }),
        ))
        .expect("enable propagation");
    let app_data = rmp_serde::to_vec_named(&MsgPackValue::Array(vec![
        MsgPackValue::Boolean(false),
        MsgPackValue::from(1_700_000_013i64),
        MsgPackValue::Boolean(true),
        MsgPackValue::from(333),
        MsgPackValue::from(999),
        MsgPackValue::Array(vec![
            MsgPackValue::from(8),
            MsgPackValue::from(2),
            MsgPackValue::from(5),
        ]),
        MsgPackValue::Map(Vec::new()),
    ]))
    .expect("encode propagation app data");

    let announce = daemon
        .handle_rpc(rpc_request(
            47,
            "announce_received",
            json!({
                "peer": "peer-propagation-limits",
                "timestamp": 1_700_000_013i64,
                "app_data_hex": hex::encode(app_data),
                "aspect": "lxmf.propagation",
                "hops": 1,
            }),
        ))
        .expect("announce received");
    assert!(announce.error.is_none());

    let peers = daemon
        .handle_rpc(RpcRequest { id: 48, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"].as_array().and_then(|rows| rows.first()).expect("peer row");
    assert_eq!(row["propagation_transfer_limit"].as_u64(), Some(333));
    assert_eq!(row["propagation_sync_limit"].as_u64(), Some(999));
    assert_eq!(row["propagation_stamp_cost"].as_u64(), Some(8));
    assert_eq!(row["propagation_stamp_cost_flexibility"].as_u64(), Some(2));
    assert_eq!(row["peering_cost"].as_u64(), Some(5));
    assert_eq!(row["type"].as_str(), Some("discovered"));
    assert_eq!(row["state"].as_u64(), Some(0));
    assert_eq!(row["sync_strategy"].as_u64(), Some(2));
    assert_eq!(row["ler"].as_u64(), Some(0));
    assert_eq!(row["str"].as_u64(), Some(0));
    assert_eq!(row["last_heard"].as_i64(), Some(1_700_000_013));
    assert_eq!(row["transfer_limit"].as_u64(), Some(333));
    assert_eq!(row["sync_limit"].as_u64(), Some(999));
    assert_eq!(row["target_stamp_cost"].as_u64(), Some(8));
    assert_eq!(row["stamp_cost_flexibility"].as_u64(), Some(2));
}

#[test]
fn announce_received_parses_propagation_peer_name_from_python_metadata() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            49,
            "propagation_enable",
            json!({
                "enabled": true,
                "autopeer": true,
            }),
        ))
        .expect("enable propagation");
    let app_data = rmp_serde::to_vec_named(&MsgPackValue::Array(vec![
        MsgPackValue::Boolean(false),
        MsgPackValue::from(1_700_000_014i64),
        MsgPackValue::Boolean(true),
        MsgPackValue::from(333),
        MsgPackValue::from(999),
        MsgPackValue::Array(vec![
            MsgPackValue::from(8),
            MsgPackValue::from(2),
            MsgPackValue::from(5),
        ]),
        MsgPackValue::Map(vec![(MsgPackValue::from(1), MsgPackValue::Binary(b"PN Alpha".to_vec()))]),
    ]))
    .expect("encode propagation app data");

    let announce = daemon
        .handle_rpc(rpc_request(
            50,
            "announce_received",
            json!({
                "peer": "peer-pn-name",
                "timestamp": 1_700_000_014i64,
                "app_data_hex": hex::encode(app_data),
                "capabilities": ["propagation"],
                "aspect": "lxmf.propagation",
                "hops": 1,
            }),
        ))
        .expect("announce received");
    assert!(announce.error.is_none());

    let peers = daemon
        .handle_rpc(RpcRequest { id: 51, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"].as_array().and_then(|rows| rows.first()).expect("peer row");
    assert_eq!(row["peer"].as_str(), Some("peer-pn-name"));
    assert_eq!(row["name"].as_str(), Some("PN Alpha"));
    assert_eq!(row["name_source"].as_str(), Some("pn_meta"));
}

#[test]
fn ticket_generate_reuses_valid_ticket_for_destination() {
    let daemon = RpcDaemon::test_instance();

    let first = daemon
        .handle_rpc(rpc_request(
            90,
            "ticket_generate",
            json!({
                "destination": "peer-ticket",
            }),
        ))
        .expect("ticket generate")
        .result
        .expect("ticket generate result");
    let second = daemon
        .handle_rpc(rpc_request(
            91,
            "ticket_generate",
            json!({
                "destination": "peer-ticket",
            }),
        ))
        .expect("ticket generate")
        .result
        .expect("ticket generate result");

    assert_eq!(first["destination"].as_str(), Some("peer-ticket"));
    assert_eq!(first["ticket"], second["ticket"]);
    assert_eq!(first["expires_at"], second["expires_at"]);
    assert_eq!(first["ticket"].as_str().map(str::len), Some(32));
    assert_eq!(first["included"], json!(true));
}

#[test]
fn ticket_generate_reuses_persisted_ticket_after_daemon_restart() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("tickets.sqlite");

    let first = {
        let store = MessagesStore::open(db_path.as_path()).expect("open store");
        let daemon = RpcDaemon::with_store(store, "ticket-node".to_string());
        daemon
            .handle_rpc(rpc_request(
                96,
                "ticket_generate",
                json!({
                    "destination": "peer-ticket-persisted",
                }),
            ))
            .expect("ticket generate")
            .result
            .expect("ticket generate result")
    };

    let second = {
        let store = MessagesStore::open(db_path.as_path()).expect("reopen store");
        let daemon = RpcDaemon::with_store(store, "ticket-node".to_string());
        daemon
            .handle_rpc(rpc_request(
                97,
                "ticket_generate",
                json!({
                    "destination": "peer-ticket-persisted",
                }),
            ))
            .expect("ticket generate")
            .result
            .expect("ticket generate result")
    };

    assert_eq!(first["included"], json!(true));
    assert_eq!(second["included"], json!(true));
    assert_eq!(first["ticket"], second["ticket"]);
    assert_eq!(first["expires_at"], second["expires_at"]);

    let store = MessagesStore::open(db_path.as_path()).expect("reopen store");
    let daemon = RpcDaemon::with_store(store, "ticket-node".to_string());
    assert_eq!(daemon.valid_issued_tickets_for("peer-ticket-persisted").len(), 1);
}

#[test]
fn ticket_generate_renews_ticket_inside_renewal_window() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("ticket-renew.sqlite");
    let destination = "peer-ticket-renew";
    let old_ticket = "000102030405060708090a0b0c0d0e0f";
    let expiring_at = now_i64() + RpcDaemon::TICKET_RENEW_SECS - 60;

    {
        let store = MessagesStore::open(db_path.as_path()).expect("open store");
        store.upsert_ticket(destination, old_ticket, expiring_at).expect("seed expiring ticket");
    }

    let store = MessagesStore::open(db_path.as_path()).expect("reopen store");
    let daemon = RpcDaemon::with_store(store, "ticket-node".to_string());
    let result = daemon
        .handle_rpc(rpc_request(
            99,
            "ticket_generate",
            json!({
                "destination": destination,
            }),
        ))
        .expect("ticket generate")
        .result
        .expect("ticket generate result");

    assert_eq!(result["included"], json!(true));
    assert_ne!(result["ticket"].as_str(), Some(old_ticket));
    assert!(result["expires_at"].as_i64().is_some_and(|expires_at| expires_at > expiring_at));
}

#[test]
fn ticket_renewal_keeps_old_unexpired_ticket_valid_like_python() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("ticket-renew-valid.sqlite");
    let destination = "peer-ticket-renew-valid";
    let old_ticket = "000102030405060708090a0b0c0d0e0f";
    let expiring_at = now_i64() + RpcDaemon::TICKET_RENEW_SECS - 60;

    {
        let store = MessagesStore::open(db_path.as_path()).expect("open store");
        store.upsert_ticket(destination, old_ticket, expiring_at).expect("seed expiring ticket");
    }

    let store = MessagesStore::open(db_path.as_path()).expect("reopen store");
    let daemon = RpcDaemon::with_store(store, "ticket-node".to_string());
    let result = daemon
        .handle_rpc(rpc_request(
            100,
            "ticket_generate",
            json!({
                "destination": destination,
            }),
        ))
        .expect("ticket generate")
        .result
        .expect("ticket generate result");
    let new_ticket = result["ticket"].as_str().expect("new ticket");
    assert_ne!(new_ticket, old_ticket);

    let valid_tickets = daemon.valid_issued_tickets_for(destination);
    let old_ticket_bytes = hex::decode(old_ticket).expect("old ticket hex");
    let new_ticket_bytes = hex::decode(new_ticket).expect("new ticket hex");
    assert_eq!(valid_tickets.len(), 2);
    assert!(valid_tickets.contains(&old_ticket_bytes));
    assert!(valid_tickets.contains(&new_ticket_bytes));
}

#[test]
fn signed_inbound_ticket_is_remembered_for_outbound_reply() {
    let daemon = RpcDaemon::test_instance();
    let expires_at = now_i64() + 3_600;
    let ticket = "00112233445566778899aabbccddeeff";

    daemon
        .accept_inbound(MessageRecord {
            id: "ticket-inbound-1".to_string(),
            source: "peer-ticket-source".to_string(),
            destination: "local".to_string(),
            title: "ticket".to_string(),
            content: "ticket body".to_string(),
            timestamp: now_i64(),
            direction: "in".to_string(),
            fields: Some(json!({
                "12": [expires_at, [0, 17, 34, 51, 68, 85, 102, 119, 136, 153, 170, 187, 204, 221, 238, 255]],
                "_lxmf": {
                    "signature_valid": true,
                },
            })),
            receipt_status: None,
        })
        .expect("accept inbound");

    let remembered =
        daemon.outbound_ticket_for("peer-ticket-source").expect("outbound ticket").expect("ticket");
    assert_eq!(remembered.ticket, ticket);
    assert_eq!(remembered.expires_at, expires_at);
}

#[test]
fn signed_inbound_ticket_accepts_python_float_expiry() {
    let daemon = RpcDaemon::test_instance();
    let expires_at = now_i64() + 3_600;
    let python_expires_at = expires_at as f64 + 0.25;
    let ticket = "00112233445566778899aabbccddeeff";

    daemon
        .accept_inbound(MessageRecord {
            id: "ticket-inbound-float-expiry".to_string(),
            source: "peer-ticket-source".to_string(),
            destination: "local".to_string(),
            title: "ticket".to_string(),
            content: "ticket body".to_string(),
            timestamp: now_i64(),
            direction: "in".to_string(),
            fields: Some(json!({
                "12": [python_expires_at, [0, 17, 34, 51, 68, 85, 102, 119, 136, 153, 170, 187, 204, 221, 238, 255]],
                "_lxmf": {
                    "signature_valid": true,
                },
            })),
            receipt_status: None,
        })
        .expect("accept inbound");

    let remembered =
        daemon.outbound_ticket_for("peer-ticket-source").expect("outbound ticket").expect("ticket");
    assert_eq!(remembered.ticket, ticket);
    assert_eq!(remembered.expires_at, expires_at + 1);
}

#[test]
fn unsigned_inbound_ticket_is_not_remembered() {
    let daemon = RpcDaemon::test_instance();
    let expires_at = now_i64() + 3_600;

    daemon
        .accept_inbound(MessageRecord {
            id: "ticket-inbound-unsigned".to_string(),
            source: "peer-ticket-source".to_string(),
            destination: "local".to_string(),
            title: "ticket".to_string(),
            content: "ticket body".to_string(),
            timestamp: now_i64(),
            direction: "in".to_string(),
            fields: Some(json!({
                "12": [expires_at, [0, 17, 34, 51, 68, 85, 102, 119, 136, 153, 170, 187, 204, 221, 238, 255]],
                "_lxmf": {
                    "signature_valid": false,
                },
            })),
            receipt_status: None,
        })
        .expect("accept inbound");

    assert!(daemon.outbound_ticket_for("peer-ticket-source").expect("outbound ticket").is_none());
}

#[test]
fn inbound_ticket_without_validated_signature_metadata_is_not_remembered_like_python() {
    let daemon = RpcDaemon::test_instance();
    let expires_at = now_i64() + 3_600;

    daemon
        .accept_inbound(MessageRecord {
            id: "ticket-inbound-unknown-signature".to_string(),
            source: "peer-ticket-source".to_string(),
            destination: "local".to_string(),
            title: "ticket".to_string(),
            content: "ticket body".to_string(),
            timestamp: now_i64(),
            direction: "in".to_string(),
            fields: Some(json!({
                "12": [expires_at, [0, 17, 34, 51, 68, 85, 102, 119, 136, 153, 170, 187, 204, 221, 238, 255]],
                "_lxmf": {
                    "signature_checked": false,
                    "signature_status": "source_identity_unknown",
                },
            })),
            receipt_status: None,
        })
        .expect("accept inbound");

    assert!(daemon.outbound_ticket_for("peer-ticket-source").expect("outbound ticket").is_none());
}

#[test]
fn signed_inbound_ticket_hex_string_is_not_remembered_like_python() {
    let daemon = RpcDaemon::test_instance();
    let expires_at = now_i64() + 3_600;

    daemon
        .accept_inbound(MessageRecord {
            id: "ticket-inbound-hex-string".to_string(),
            source: "peer-ticket-source".to_string(),
            destination: "local".to_string(),
            title: "ticket".to_string(),
            content: "ticket body".to_string(),
            timestamp: now_i64(),
            direction: "in".to_string(),
            fields: Some(json!({
                "12": [expires_at, "00112233445566778899aabbccddeeff"],
                "_lxmf": {
                    "signature_valid": true,
                },
            })),
            receipt_status: None,
        })
        .expect("accept inbound");

    assert!(daemon.outbound_ticket_for("peer-ticket-source").expect("outbound ticket").is_none());
}

#[test]
fn ticket_generate_suppresses_recently_delivered_ticket() {
    let daemon = RpcDaemon::test_instance();

    daemon.mark_ticket_delivered("peer-ticket-recent");

    let result = daemon
        .handle_rpc(rpc_request(
            92,
            "ticket_generate",
            json!({
                "destination": "peer-ticket-recent",
            }),
        ))
        .expect("ticket generate")
        .result
        .expect("ticket generate result");

    assert_eq!(result["destination"].as_str(), Some("peer-ticket-recent"));
    assert_eq!(result["included"], json!(false));
    assert_eq!(result["ticket"], JsonValue::Null);
    assert_eq!(result["expires_at"], JsonValue::Null);
    assert_eq!(result["reason"].as_str(), Some("ticket_interval"));
}

#[test]
fn ticket_generate_suppresses_recent_delivery_after_daemon_restart() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("ticket-deliveries.sqlite");

    {
        let store = MessagesStore::open(db_path.as_path()).expect("open store");
        let daemon = RpcDaemon::with_store(store, "ticket-node".to_string());
        daemon.mark_ticket_delivered("peer-ticket-restart-interval");
    }

    let result = {
        let store = MessagesStore::open(db_path.as_path()).expect("reopen store");
        let daemon = RpcDaemon::with_store(store, "ticket-node".to_string());
        daemon
            .handle_rpc(rpc_request(
                98,
                "ticket_generate",
                json!({
                    "destination": "peer-ticket-restart-interval",
                }),
            ))
            .expect("ticket generate")
            .result
            .expect("ticket generate result")
    };

    assert_eq!(result["included"], json!(false));
    assert_eq!(result["reason"].as_str(), Some("ticket_interval"));
}

#[test]
fn delivered_include_ticket_message_starts_ticket_interval() {
    let daemon = RpcDaemon::test_instance();

    daemon
        .handle_rpc(rpc_request(
            93,
            "sdk_send_v2",
            json!({
                "id": "ticket-msg-1",
                "source": "local",
                "destination": "peer-ticket-delivered",
                "title": "ticket",
                "content": "ticket body",
                "method": "direct",
                "include_ticket": true,
            }),
        ))
        .expect("send message");
    daemon
        .handle_rpc(rpc_request(
            94,
            "record_receipt",
            json!({
                "message_id": "ticket-msg-1",
                "status": "delivered",
            }),
        ))
        .expect("record delivery");

    let result = daemon
        .handle_rpc(rpc_request(
            95,
            "ticket_generate",
            json!({
                "destination": "peer-ticket-delivered",
            }),
        ))
        .expect("ticket generate")
        .result
        .expect("ticket generate result");

    assert_eq!(result["included"], json!(false));
    assert_eq!(result["reason"].as_str(), Some("ticket_interval"));
}

#[test]
fn autopeered_announce_records_propagation_peer_state() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            45,
            "propagation_enable",
            json!({
                "enabled": true,
                "autopeer": true,
                "autopeer_maxdepth": 2,
                "remote_peering_cost_max": 8,
            }),
        ))
        .expect("enable propagation");
    let app_data = rmp_serde::to_vec_named(&MsgPackValue::Array(vec![
        MsgPackValue::Boolean(false),
        MsgPackValue::from(1_700_000_100i64),
        MsgPackValue::Boolean(true),
        MsgPackValue::from(512),
        MsgPackValue::from(2048),
        MsgPackValue::Array(vec![
            MsgPackValue::from(4),
            MsgPackValue::from(1),
            MsgPackValue::from(7),
        ]),
        MsgPackValue::Map(Vec::new()),
    ]))
    .expect("encode propagation app data");

    daemon
        .accept_announce_with_metadata(
            "peer-auto".to_string(),
            1_700_000_100,
            Some("Peer Auto".to_string()),
            Some("announce".to_string()),
            Some(hex::encode(app_data)),
            Some(vec!["propagation".to_string()]),
            None,
            None,
            None,
            Some(4),
            Some(Some(1)),
            Some(Some(7)),
            None,
            Some(1),
            None,
            None,
            None,
            None,
        )
        .expect("accept announce");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 46, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"].as_array().and_then(|rows| rows.first()).expect("peer row");
    assert_eq!(row["peer"].as_str(), Some("peer-auto"));
    assert_eq!(row["peer_type"].as_str(), Some("auto"));
    assert_eq!(row["peering_timebase"].as_i64(), Some(1_700_000_100));
    assert_eq!(row["propagation_transfer_limit"].as_u64(), Some(512));
    assert_eq!(row["propagation_sync_limit"].as_u64(), Some(2048));
    assert_eq!(row["propagation_stamp_cost"].as_u64(), Some(4));
    assert_eq!(row["propagation_stamp_cost_flexibility"].as_u64(), Some(1));
    assert_eq!(row["peering_cost"].as_u64(), Some(7));
}

#[test]
fn stale_announce_does_not_regress_propagation_peer_state() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            47,
            "propagation_enable",
            json!({
                "enabled": true,
                "autopeer": true,
            }),
        ))
        .expect("enable propagation");

    daemon
        .accept_announce_with_metadata(
            "peer-auto".to_string(),
            1_700_000_200,
            Some("New Peer".to_string()),
            Some("announce".to_string()),
            None,
            Some(vec!["propagation".to_string()]),
            None,
            None,
            None,
            Some(5),
            Some(Some(2)),
            Some(Some(6)),
            None,
            Some(1),
            None,
            None,
            None,
            None,
        )
        .expect("accept fresh announce");
    daemon
        .accept_announce_with_metadata(
            "peer-auto".to_string(),
            1_700_000_150,
            Some("Old Peer".to_string()),
            Some("announce".to_string()),
            None,
            Some(vec!["propagation".to_string()]),
            None,
            None,
            None,
            Some(2),
            Some(Some(0)),
            Some(Some(3)),
            None,
            Some(1),
            None,
            None,
            None,
            None,
        )
        .expect("accept stale announce");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 48, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"].as_array().and_then(|rows| rows.first()).expect("peer row");
    assert_eq!(row["name"].as_str(), Some("New Peer"));
    assert_eq!(row["peering_timebase"].as_i64(), Some(1_700_000_200));
    assert_eq!(row["propagation_stamp_cost"].as_u64(), Some(5));
    assert_eq!(row["propagation_stamp_cost_flexibility"].as_u64(), Some(2));
    assert_eq!(row["peering_cost"].as_u64(), Some(6));

    let announces = daemon
        .handle_rpc(RpcRequest { id: 49, method: "list_announces".to_string(), params: None })
        .expect("list announces")
        .result
        .expect("list announces result");
    let rows = announces["announces"].as_array().expect("announce rows");
    assert_eq!(rows.first().and_then(|row| row["timestamp"].as_i64()), Some(1_700_000_200));
    assert_eq!(rows.get(1).and_then(|row| row["timestamp"].as_i64()), Some(1_700_000_150));
}

#[test]
fn discovered_announce_bursts_do_not_collapse_in_announce_log() {
    let daemon = RpcDaemon::test_instance();
    let timestamp = 1_700_000_250;

    for idx in 0..4 {
        daemon
            .accept_announce_with_metadata(
                "peer-discovered".to_string(),
                timestamp,
                Some(format!("Peer Discovered {idx}")),
                Some("announce".to_string()),
                None,
                Some(vec!["propagation".to_string()]),
                None,
                None,
                None,
                Some(3),
                Some(Some(1)),
                Some(Some(4)),
                None,
                Some(1),
                None,
                None,
                None,
                None,
            )
            .expect("accept discovered announce");
    }

    let announces = daemon
        .handle_rpc(RpcRequest { id: 50, method: "list_announces".to_string(), params: None })
        .expect("list announces")
        .result
        .expect("list announces result");
    let rows = announces["announces"].as_array().expect("announce rows");
    let matching = rows
        .iter()
        .filter(|row| row["peer"].as_str() == Some("peer-discovered"))
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 4, "same-second discovered announces must remain distinct");
    let unique_ids = matching
        .iter()
        .filter_map(|row| row["id"].as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(unique_ids.len(), 4, "announce log IDs must be unique for burst traffic");

    let legacy_event_ids = std::iter::from_fn(|| daemon.take_event())
        .filter(|event| event.event_type == "announce_received")
        .filter_map(|event| event.payload["id"].as_str().map(str::to_string))
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        legacy_event_ids.len(),
        4,
        "daemon event queue must expose unique announce IDs for burst traffic"
    );

    let events = daemon
        .handle_rpc(rpc_request(51, "sdk_poll_events_v2", json!({ "cursor": null, "max": 20 })))
        .expect("poll sdk events")
        .result
        .expect("sdk events result");
    let event_rows = events["events"].as_array().expect("event rows");
    let announce_event_ids = event_rows
        .iter()
        .filter(|row| row["event_type"].as_str() == Some("announce_received"))
        .filter_map(|row| row["payload"]["id"].as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        announce_event_ids.len(),
        4,
        "SDK announce events must expose unique IDs for burst traffic"
    );
}

#[test]
fn peering_cost_policy_blocks_and_breaks_autopeers() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            50,
            "propagation_enable",
            json!({
                "enabled": true,
                "autopeer": true,
                "remote_peering_cost_max": 5,
            }),
        ))
        .expect("enable propagation");

    daemon
        .accept_announce_with_metadata(
            "peer-auto".to_string(),
            1_700_000_300,
            None,
            None,
            None,
            Some(vec!["propagation".to_string()]),
            None,
            None,
            None,
            Some(3),
            Some(Some(1)),
            Some(Some(4)),
            None,
            Some(1),
            None,
            None,
            None,
            None,
        )
        .expect("accept initial announce");

    daemon
        .accept_announce_with_metadata(
            "peer-auto".to_string(),
            1_700_000_301,
            None,
            None,
            None,
            Some(vec!["propagation".to_string()]),
            None,
            None,
            None,
            Some(3),
            Some(Some(1)),
            Some(Some(9)),
            None,
            Some(1),
            None,
            None,
            None,
            None,
        )
        .expect("accept high-cost announce");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 51, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    assert_eq!(peers["peers"].as_array().map(|rows| rows.len()), Some(0));
}

#[test]
fn peer_activity_updates_runtime_counters() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            50,
            "peer_sync",
            json!({
                "peer": "peer-runtime",
            }),
        ))
        .expect("peer sync");

    daemon.record_inbound_peer_activity("peer-runtime", 120);
    daemon.record_outbound_peer_activity("peer-runtime", 80, true);
    daemon.record_outbound_peer_activity("peer-runtime", 40, false);

    let peers = daemon
        .handle_rpc(RpcRequest { id: 51, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"].as_array().and_then(|rows| rows.first()).expect("peer row");
    assert_eq!(row["peer"].as_str(), Some("peer-runtime"));
    assert_eq!(row["rx_bytes"].as_u64(), Some(120));
    assert_eq!(row["tx_bytes"].as_u64(), Some(120));
    assert_eq!(row["sync_backoff"].as_u64(), Some(12 * 60));
    assert_eq!(
        row["next_sync_attempt"].as_i64(),
        Some(row["last_sync_attempt"].as_i64().expect("last sync attempt") + 12 * 60)
    );
    assert!(row["acceptance_rate"].as_f64().is_some_and(|value| value < 1.0));
}

#[test]
fn delivered_peer_activity_updates_last_heard_like_python() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(51, "peer_sync", json!({ "peer": "peer-delivered-heard" })))
        .expect("peer sync");

    daemon.record_outbound_peer_activity("peer-delivered-heard", 64, true);

    let peers = daemon
        .handle_rpc(RpcRequest { id: 52, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"].as_array().and_then(|rows| rows.first()).expect("peer row");
    let last_seen = row["last_seen"].as_i64().expect("last_seen");
    assert!(last_seen > 0);
    assert_eq!(row["last_heard"].as_i64(), Some(last_seen));
    assert_eq!(row["last_sync_attempt"].as_i64(), Some(last_seen));
}

#[test]
fn sent_peer_activity_does_not_mark_peer_heard_like_python() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            51,
            "propagation_enable",
            json!({
                "enabled": true,
                "static_peers": ["peer-sent-only"],
            }),
        ))
        .expect("enable static peer");

    daemon.record_outbound_peer_sent("peer-sent-only", 64);

    let peers = daemon
        .handle_rpc(RpcRequest { id: 52, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"].as_array().and_then(|rows| rows.first()).expect("peer row");
    assert_eq!(row["peer"].as_str(), Some("peer-sent-only"));
    assert_eq!(row["tx_bytes"].as_u64(), Some(64));
    assert_eq!(row["alive"].as_bool(), Some(false));
    assert_eq!(row["last_heard"].as_i64(), Some(0));
    assert_eq!(row["sync_backoff"].as_u64(), Some(0));
    assert_eq!(row["acceptance_rate"].as_f64(), Some(0.0));
    assert!(row["last_sync_attempt"].as_i64().is_some_and(|value| value > 0));
}

#[test]
fn failed_peer_activity_does_not_mark_unheard_static_peer_alive() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            52,
            "propagation_enable",
            json!({
                "enabled": true,
                "static_peers": ["peer-static-failed"],
            }),
        ))
        .expect("enable static peer");

    daemon.record_outbound_peer_activity("peer-static-failed", 32, false);

    let peers = daemon
        .handle_rpc(RpcRequest { id: 53, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"].as_array().and_then(|rows| rows.first()).expect("peer row");
    assert_eq!(row["peer"].as_str(), Some("peer-static-failed"));
    assert_eq!(row["alive"].as_bool(), Some(false));
    assert_eq!(row["last_heard"].as_i64(), Some(0));
    assert_eq!(row["sync_backoff"].as_u64(), Some(12 * 60));
}

#[test]
fn new_peer_acceptance_rate_matches_python_zero_offer_default() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(52, "peer_sync", json!({ "peer": "peer-zero-offers" })))
        .expect("peer sync");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 53, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"].as_array().and_then(|rows| rows.first()).expect("peer row");
    assert_eq!(row["peer"].as_str(), Some("peer-zero-offers"));
    assert_eq!(row["acceptance_rate"].as_f64(), Some(0.0));
}

#[test]
fn peer_sync_without_offers_preserves_failure_backoff() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(52, "peer_sync", json!({ "peer": "peer-backoff-no-offers" })))
        .expect("initial peer sync");
    daemon.record_outbound_peer_activity("peer-backoff-no-offers", 64, false);

    let before = daemon
        .handle_rpc(RpcRequest { id: 53, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let before_row = before["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-backoff-no-offers"))
        .expect("peer row");
    let sync_backoff = before_row["sync_backoff"].as_u64().expect("sync backoff");
    let next_sync_attempt =
        before_row["next_sync_attempt"].as_i64().expect("next sync attempt");
    assert!(sync_backoff > 0);
    assert!(next_sync_attempt > 0);

    let result = daemon
        .handle_rpc(rpc_request(54, "peer_sync", json!({ "peer": "peer-backoff-no-offers" })))
        .expect("no-offer peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["propagation"]["offered"].as_u64(), Some(0));

    let after = daemon
        .handle_rpc(RpcRequest { id: 55, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let after_row = after["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-backoff-no-offers"))
        .expect("peer row");
    assert_eq!(after_row["sync_backoff"].as_u64(), Some(sync_backoff));
    assert_eq!(after_row["next_sync_attempt"].as_i64(), Some(next_sync_attempt));
    assert_eq!(after_row["alive"].as_bool(), Some(false));
}

#[test]
fn peer_sync_during_backoff_postpones_skipped_offers() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(52, "peer_sync", json!({ "peer": "peer-backoff-skipped" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-backoff-skipped").expect("peer record");
        peer.propagation_sync_limit = Some(1_000);
        peer.peering_timebase = 1_700_000_000;
        peer.network_distance = 3;
    }
    let previous_transfer = PropagationEntryRecord {
        transient_id: "ed".repeat(32),
        destination: "18".repeat(16),
        payload_hex: "19".repeat(40),
        received_at: 1_700_000_613,
        size_bytes: 40,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&previous_transfer).expect("store previous transfer");
    daemon
        .store
        .mark_peer_unhandled_propagation(
            "peer-backoff-skipped",
            previous_transfer.transient_id.as_str(),
        )
        .expect("mark previous transfer unhandled");
    daemon
        .handle_rpc(rpc_request(53, "peer_sync", json!({ "peer": "peer-backoff-skipped" })))
        .expect("peer sync with previous transfer");
    daemon.record_outbound_peer_activity("peer-backoff-skipped", 64, false);
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-backoff-skipped").expect("peer record");
        peer.propagation_sync_limit = Some(24);
    }
    let entry = PropagationEntryRecord {
        transient_id: "ee".repeat(32),
        destination: "18".repeat(16),
        payload_hex: "18".repeat(20),
        received_at: 1_700_000_614,
        size_bytes: 20,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-backoff-skipped", entry.transient_id.as_str())
        .expect("mark unhandled");

    let before = daemon
        .handle_rpc(RpcRequest { id: 53, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let before_row = before["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-backoff-skipped"))
        .expect("peer row");
    let sync_backoff = before_row["sync_backoff"].as_u64().expect("sync backoff");
    let next_sync_attempt =
        before_row["next_sync_attempt"].as_i64().expect("next sync attempt");
    assert!(sync_backoff > 0);
    assert!(next_sync_attempt > 0);

    let result = daemon
        .handle_rpc(rpc_request(54, "peer_sync", json!({ "peer": "peer-backoff-skipped" })))
        .expect("skipped peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["synced"].as_bool(), Some(false));
    assert_eq!(result["postponed"].as_bool(), Some(true));
    assert_eq!(result["postpone_reason"].as_str(), Some("backoff"));
    assert_eq!(result["state"].as_u64(), Some(0));
    assert_eq!(result["sync_strategy"].as_u64(), Some(2));
    assert_eq!(result["ler"].as_u64(), Some(0));
    assert_eq!(result["network_distance"].as_u64(), Some(3));
    assert_eq!(result["peering_timebase"].as_i64(), Some(1_700_000_000));
    assert_eq!(result["rx_bytes"].as_u64(), Some(0));
    assert_eq!(result["tx_bytes"].as_u64(), Some(104));
    assert_eq!(result["sync_transfer_rate"].as_f64(), Some(0.0));
    assert_eq!(result["str"].as_u64(), Some(0));
    assert!(result["last_heard"].as_i64().is_some_and(|value| value > 0));
    assert_eq!(result["propagation"]["synced"].as_bool(), Some(false));
    assert_eq!(result["propagation"]["postponed"].as_bool(), Some(true));
    assert_eq!(result["propagation"]["postpone_reason"].as_str(), Some("backoff"));
    assert_eq!(result["propagation"]["offered"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["handled"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["skipped"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["transfer_limited"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["transfer_limited_bytes"].as_u64(), Some(0));
    assert_eq!(
        result["propagation"]["transfer_limited_ids"]
            .as_array()
            .expect("transfer limited ids"),
        &[] as &[JsonValue]
    );

    let after = daemon
        .handle_rpc(RpcRequest { id: 55, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let after_row = after["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-backoff-skipped"))
        .expect("peer row");
    assert_eq!(after_row["sync_backoff"].as_u64(), Some(sync_backoff));
    assert_eq!(after_row["next_sync_attempt"].as_i64(), Some(next_sync_attempt));
    assert_eq!(after_row["sync_transfer_rate"].as_f64(), Some(0.0));
    assert_eq!(after_row["str"].as_u64(), Some(0));
}

#[test]
fn peer_sync_postpones_offers_until_stamp_policy_is_known() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(52, "peer_sync", json!({ "peer": "peer-missing-stamp-policy" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-missing-stamp-policy").expect("peer record");
        peer.propagation_sync_limit = Some(1_000);
        peer.propagation_stamp_cost = None;
        peer.propagation_stamp_cost_flexibility = None;
        peer.peering_cost = None;
    }
    let entry = PropagationEntryRecord {
        transient_id: "eb".repeat(32),
        destination: "1b".repeat(16),
        payload_hex: "1b".repeat(20),
        received_at: 1_700_000_617,
        size_bytes: 20,
        stamp_value: Some(1),
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-missing-stamp-policy", entry.transient_id.as_str())
        .expect("mark unhandled");

    let result = daemon
        .handle_rpc(rpc_request(54, "peer_sync", json!({ "peer": "peer-missing-stamp-policy" })))
        .expect("policy-gated peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["synced"].as_bool(), Some(false));
    assert_eq!(result["postponed"].as_bool(), Some(true));
    assert_eq!(result["postpone_reason"].as_str(), Some("stamp_policy"));
    assert_eq!(result["propagation"]["synced"].as_bool(), Some(false));
    assert_eq!(result["propagation"]["postponed"].as_bool(), Some(true));
    assert_eq!(
        result["propagation"]["postpone_reason"].as_str(),
        Some("stamp_policy")
    );
    assert_eq!(result["propagation"]["offered"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["handled"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["skipped"].as_u64(), Some(0));

    let unhandled = daemon
        .store
        .list_peer_unhandled_propagation("peer-missing-stamp-policy")
        .expect("list unhandled");
    assert_eq!(unhandled.len(), 1);
    assert_eq!(unhandled[0].transient_id, entry.transient_id);
}

#[test]
fn peer_sync_postpones_stamped_offers_until_peering_key_is_ready() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(52, "peer_sync", json!({ "peer": "peer-missing-peering-key" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-missing-peering-key").expect("peer record");
        peer.propagation_sync_limit = Some(1_000);
        peer.propagation_stamp_cost = Some(1);
        peer.propagation_stamp_cost_flexibility = Some(1);
        peer.peering_cost = Some(1);
    }
    let entry = PropagationEntryRecord {
        transient_id: "ec".repeat(32),
        destination: "1c".repeat(16),
        payload_hex: "1c".repeat(20),
        received_at: 1_700_000_618,
        size_bytes: 20,
        stamp_value: Some(1),
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-missing-peering-key", entry.transient_id.as_str())
        .expect("mark unhandled");

    let result = daemon
        .handle_rpc(rpc_request(54, "peer_sync", json!({ "peer": "peer-missing-peering-key" })))
        .expect("peering-key-gated peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["synced"].as_bool(), Some(false));
    assert_eq!(result["postponed"].as_bool(), Some(true));
    assert_eq!(result["postpone_reason"].as_str(), Some("peering_key"));
    assert_eq!(result["peering_key"], JsonValue::Null);
    assert_eq!(result["propagation"]["offered"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["handled"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["skipped"].as_u64(), Some(0));

    let unhandled = daemon
        .store
        .list_peer_unhandled_propagation("peer-missing-peering-key")
        .expect("list unhandled");
    assert_eq!(unhandled.len(), 1);
    assert_eq!(unhandled[0].transient_id, entry.transient_id);
}

#[test]
fn repeated_skipped_peer_sync_is_postponed_until_backoff_expires() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(52, "peer_sync", json!({ "peer": "peer-skipped-repeat" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-skipped-repeat").expect("peer record");
        peer.propagation_sync_limit = Some(24);
    }
    let entry = PropagationEntryRecord {
        transient_id: "ea".repeat(32),
        destination: "1a".repeat(16),
        payload_hex: "1a".repeat(20),
        received_at: 1_700_000_616,
        size_bytes: 20,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-skipped-repeat", entry.transient_id.as_str())
        .expect("mark unhandled");

    daemon
        .handle_rpc(rpc_request(53, "peer_sync", json!({ "peer": "peer-skipped-repeat" })))
        .expect("first skipped peer sync");
    let first = daemon
        .handle_rpc(RpcRequest { id: 54, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let first_row = first["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-skipped-repeat"))
        .expect("peer row");
    let first_attempt = first_row["last_sync_attempt"].as_i64().expect("first attempt");
    assert_eq!(first_row["sync_backoff"].as_u64(), Some(12 * 60));

    let second_result = daemon
        .handle_rpc(rpc_request(55, "peer_sync", json!({ "peer": "peer-skipped-repeat" })))
        .expect("second skipped peer sync")
        .result
        .expect("second peer sync result");
    assert_eq!(second_result["synced"].as_bool(), Some(false));
    assert_eq!(second_result["postponed"].as_bool(), Some(true));
    assert_eq!(second_result["postpone_reason"].as_str(), Some("backoff"));
    assert_eq!(second_result["propagation"]["offered"].as_u64(), Some(0));
    assert_eq!(second_result["propagation"]["handled"].as_u64(), Some(0));
    assert_eq!(second_result["propagation"]["skipped"].as_u64(), Some(0));

    let second = daemon
        .handle_rpc(RpcRequest { id: 56, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let second_row = second["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-skipped-repeat"))
        .expect("peer row");
    assert_eq!(second_row["sync_backoff"].as_u64(), Some(12 * 60));
    assert!(second_row["last_sync_attempt"].as_i64().is_some_and(|value| value >= first_attempt));
    assert_eq!(
        second_row["next_sync_attempt"].as_i64(),
        first_row["next_sync_attempt"].as_i64()
    );
}

#[test]
fn peer_sync_with_only_skipped_offers_schedules_initial_backoff() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(52, "peer_sync", json!({ "peer": "peer-skipped-initial" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-skipped-initial").expect("peer record");
        peer.propagation_sync_limit = Some(24);
    }
    let entry = PropagationEntryRecord {
        transient_id: "ef".repeat(32),
        destination: "19".repeat(16),
        payload_hex: "19".repeat(20),
        received_at: 1_700_000_615,
        size_bytes: 20,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-skipped-initial", entry.transient_id.as_str())
        .expect("mark unhandled");

    let result = daemon
        .handle_rpc(rpc_request(53, "peer_sync", json!({ "peer": "peer-skipped-initial" })))
        .expect("skipped peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["propagation"]["offered"].as_u64(), Some(1));
    assert_eq!(result["propagation"]["handled"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["skipped"].as_u64(), Some(1));
    assert_eq!(result["acceptance_rate"].as_f64(), Some(0.0));

    let peers = daemon
        .handle_rpc(RpcRequest { id: 54, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-skipped-initial"))
        .expect("peer row");
    assert_eq!(row["sync_backoff"].as_u64(), Some(12 * 60));
    assert_eq!(
        row["next_sync_attempt"].as_i64(),
        Some(row["last_sync_attempt"].as_i64().expect("last sync attempt") + 12 * 60)
    );
}

#[test]
fn peer_sync_result_and_event_report_backoff_schedule() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(55, "peer_sync", json!({ "peer": "peer-backoff-report" })))
        .expect("initial peer sync");
    daemon.event_queue.lock().expect("event_queue mutex poisoned").clear();
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-backoff-report").expect("peer record");
        peer.propagation_sync_limit = Some(24);
    }
    let entry = PropagationEntryRecord {
        transient_id: "ba".repeat(32),
        destination: "19".repeat(16),
        payload_hex: "19".repeat(20),
        received_at: 1_700_000_618,
        size_bytes: 20,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-backoff-report", entry.transient_id.as_str())
        .expect("mark unhandled");

    let result = daemon
        .handle_rpc(rpc_request(56, "peer_sync", json!({ "peer": "peer-backoff-report" })))
        .expect("skipped peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["sync_backoff"].as_u64(), Some(12 * 60));
    let last_sync_attempt = result["last_sync_attempt"].as_i64().expect("last sync attempt");
    let last_heard = result["last_heard"].as_i64().expect("last heard");
    assert!(last_sync_attempt > 0);
    assert!(last_heard > 0);
    assert_eq!(result["next_sync_attempt"].as_i64(), Some(last_sync_attempt + 12 * 60));

    let event = daemon
        .event_queue
        .lock()
        .expect("event_queue mutex poisoned")
        .iter()
        .rev()
        .find(|event| event.event_type == "peer_sync")
        .cloned()
        .expect("peer sync event");
    assert_eq!(event.payload["sync_backoff"].as_u64(), Some(12 * 60));
    assert_eq!(event.payload["last_sync_attempt"].as_i64(), Some(last_sync_attempt));
    assert_eq!(event.payload["last_heard"].as_i64(), Some(last_heard));
    assert_eq!(event.payload["next_sync_attempt"].as_i64(), Some(last_sync_attempt + 12 * 60));
}

#[test]
fn list_peers_exposes_python_style_message_counters() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(52, "peer_sync", json!({ "peer": "peer-message-stats" })))
        .expect("peer sync");
    daemon
        .accept_inbound(MessageRecord {
            id: "peer-message-stats-in".to_string(),
            source: "peer-message-stats".to_string(),
            destination: "local".to_string(),
            title: "title".to_string(),
            content: "body".to_string(),
            timestamp: 1_700_000_600,
            direction: "in".to_string(),
            fields: None,
            receipt_status: None,
        })
        .expect("accept inbound");
    daemon
        .accept_inbound(MessageRecord {
            id: "peer-message-stats-out".to_string(),
            source: "local".to_string(),
            destination: "peer-message-stats".to_string(),
            title: "title".to_string(),
            content: "body".to_string(),
            timestamp: 1_700_000_601,
            direction: "out".to_string(),
            fields: None,
            receipt_status: None,
        })
        .expect("store outbound");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 54, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-message-stats"))
        .expect("peer row");
    assert_eq!(row["messages"]["outgoing"].as_u64(), Some(1));
    assert_eq!(row["messages"]["incoming"].as_u64(), Some(1));
    assert_eq!(row["messages"]["offered"].as_u64(), Some(1));
    assert_eq!(row["messages"]["unhandled"].as_u64(), Some(1));
}

#[test]
fn peer_sync_marks_unhandled_propagation_entries_handled() {
    let daemon = RpcDaemon::test_instance();
    let entry = PropagationEntryRecord {
        transient_id: "ab".repeat(32),
        destination: "12".repeat(16),
        payload_hex: "12".repeat(16),
        received_at: 1_700_000_605,
        size_bytes: 16,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store propagation entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-propagation-sync", entry.transient_id.as_str())
        .expect("mark propagation unhandled");

    daemon
        .handle_rpc(rpc_request(55, "peer_sync", json!({ "peer": "peer-propagation-sync" })))
        .expect("peer sync");

    assert!(
        daemon
            .store
            .list_peer_unhandled_propagation("peer-propagation-sync")
            .expect("list unhandled")
            .is_empty()
    );
    assert_eq!(
        daemon
            .store
            .list_peer_handled_propagation_ids("peer-propagation-sync")
            .expect("list handled"),
        vec![entry.transient_id]
    );
}

#[test]
fn peer_sync_drops_stale_unhandled_propagation_marks() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(55, "peer_sync", json!({ "peer": "peer-stale-propagation" })))
        .expect("initial peer sync");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-stale-propagation", "fa".repeat(32).as_str())
        .expect("mark stale unhandled");

    let before = daemon
        .handle_rpc(RpcRequest { id: 56, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let before_row = before["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-stale-propagation"))
        .expect("peer row");
    assert_eq!(before_row["messages"]["unhandled"].as_u64(), Some(0));
    assert_eq!(
        before_row["messages"]["unhandled_ids"].as_array().expect("message unhandled ids"),
        &[] as &[JsonValue]
    );
    assert_eq!(
        before_row["unhandled_ids"].as_array().expect("top-level unhandled ids"),
        &[] as &[JsonValue]
    );

    let result = daemon
        .handle_rpc(rpc_request(57, "peer_sync", json!({ "peer": "peer-stale-propagation" })))
        .expect("peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["propagation"]["offered"].as_u64(), Some(0));

    let after = daemon
        .handle_rpc(RpcRequest { id: 58, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let after_row = after["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-stale-propagation"))
        .expect("peer row");
    assert_eq!(after_row["messages"]["unhandled"].as_u64(), Some(0));
    assert_eq!(after_row["messages"]["offered"].as_u64(), Some(0));
    assert_eq!(
        after_row["messages"]["unhandled_ids"].as_array().expect("message unhandled ids"),
        &[] as &[JsonValue]
    );
    assert_eq!(
        after_row["unhandled_ids"].as_array().expect("top-level unhandled ids"),
        &[] as &[JsonValue]
    );
}

#[test]
fn list_peers_ignores_stale_handled_propagation_marks() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(55, "peer_sync", json!({ "peer": "peer-stale-handled" })))
        .expect("initial peer sync");
    daemon
        .store
        .mark_peer_handled_propagation("peer-stale-handled", "fb".repeat(32).as_str())
        .expect("mark stale handled");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 56, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-stale-handled"))
        .expect("peer row");
    assert_eq!(row["messages"]["offered"].as_u64(), Some(0));
    assert_eq!(row["messages"]["unhandled"].as_u64(), Some(0));
    assert_eq!(row["messages"]["offered_bytes"].as_u64(), Some(0));
    assert_eq!(row["messages"]["unhandled_bytes"].as_u64(), Some(0));
    assert_eq!(
        row["messages"]["handled_ids"].as_array().expect("message handled ids"),
        &[] as &[JsonValue]
    );
    assert_eq!(
        row["handled_ids"].as_array().expect("top-level handled ids"),
        &[] as &[JsonValue]
    );
}

#[test]
fn peer_sync_applies_per_peer_propagation_sync_limit() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(56, "peer_sync", json!({ "peer": "peer-sync-budget" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-sync-budget").expect("peer record");
        peer.propagation_sync_limit = Some((24 + 20 + 32 + 16 + 1) as u32);
    }

    let small = PropagationEntryRecord {
        transient_id: "b1".repeat(32),
        destination: "15".repeat(16),
        payload_hex: "15".repeat(20),
        received_at: 1_700_000_608,
        size_bytes: 20,
        stamp_value: None,
    };
    let large = PropagationEntryRecord {
        transient_id: "b2".repeat(32),
        destination: "15".repeat(16),
        payload_hex: "15".repeat(100),
        received_at: 1_700_000_609,
        size_bytes: 100,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&small).expect("store small entry");
    daemon.store.upsert_propagation_entry(&large).expect("store large entry");
    for entry in [&small, &large] {
        daemon
            .store
            .mark_peer_unhandled_propagation("peer-sync-budget", entry.transient_id.as_str())
            .expect("mark unhandled");
    }

    daemon
        .handle_rpc(rpc_request(57, "peer_sync", json!({ "peer": "peer-sync-budget" })))
        .expect("budgeted peer sync");

    let handled = daemon
        .store
        .list_peer_handled_propagation_ids("peer-sync-budget")
        .expect("handled ids");
    assert_eq!(handled, vec![small.transient_id]);
    let pending = daemon
        .store
        .list_peer_unhandled_propagation("peer-sync-budget")
        .expect("pending propagation");
    assert_eq!(pending, vec![large]);
}

#[test]
fn peer_sync_skips_entry_at_exact_sync_limit_like_python() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(56, "peer_sync", json!({ "peer": "peer-sync-equal-budget" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-sync-equal-budget").expect("peer record");
        peer.propagation_sync_limit = Some((24 + 20 + 16) as u32);
    }

    let entry = PropagationEntryRecord {
        transient_id: "b3".repeat(32),
        destination: "15".repeat(16),
        payload_hex: "15".repeat(20),
        received_at: 1_700_000_609,
        size_bytes: 20,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store propagation entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-sync-equal-budget", entry.transient_id.as_str())
        .expect("mark unhandled");

    let result = daemon
        .handle_rpc(rpc_request(57, "peer_sync", json!({ "peer": "peer-sync-equal-budget" })))
        .expect("budgeted peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["propagation"]["handled"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["skipped"].as_u64(), Some(1));
    assert_eq!(result["propagation"]["offered"].as_u64(), Some(1));
    assert_eq!(
        result["propagation"]["skipped_ids"].as_array().expect("skipped ids"),
        &[json!(entry.transient_id.as_str())]
    );

    assert!(
        daemon
            .store
            .list_peer_handled_propagation_ids("peer-sync-equal-budget")
            .expect("handled ids")
            .is_empty()
    );
    let pending = daemon
        .store
        .list_peer_unhandled_propagation("peer-sync-equal-budget")
        .expect("pending propagation");
    assert_eq!(pending, vec![entry]);
}

#[test]
fn peer_sync_applies_python_per_message_overhead_for_sync_limit() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(56, "peer_sync", json!({ "peer": "peer-sync-overhead" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-sync-overhead").expect("peer record");
        peer.propagation_sync_limit = Some((24 + 40 + 16 + 1) as u32);
    }

    let entry = PropagationEntryRecord {
        transient_id: "b4".repeat(32),
        destination: "15".repeat(16),
        payload_hex: "15".repeat(40),
        received_at: 1_700_000_610,
        size_bytes: 40,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store propagation entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-sync-overhead", entry.transient_id.as_str())
        .expect("mark unhandled");

    let result = daemon
        .handle_rpc(rpc_request(57, "peer_sync", json!({ "peer": "peer-sync-overhead" })))
        .expect("budgeted peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["propagation"]["handled"].as_u64(), Some(1));
    assert_eq!(result["propagation"]["skipped"].as_u64(), Some(0));
    assert_eq!(
        result["propagation"]["handled_ids"].as_array().expect("handled ids"),
        &[json!(entry.transient_id.as_str())]
    );

    let handled = daemon
        .store
        .list_peer_handled_propagation_ids("peer-sync-overhead")
        .expect("handled ids");
    assert_eq!(handled, vec![entry.transient_id]);
}

#[test]
fn peer_sync_uses_transfer_limit_when_sync_limit_is_absent() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(58, "peer_sync", json!({ "peer": "peer-transfer-budget" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-transfer-budget").expect("peer record");
        peer.propagation_transfer_limit = Some((24 + 20 + 32 + 16 + 1) as u32);
        peer.propagation_sync_limit = None;
    }

    let small = PropagationEntryRecord {
        transient_id: "c1".repeat(32),
        destination: "16".repeat(16),
        payload_hex: "16".repeat(20),
        received_at: 1_700_000_610,
        size_bytes: 20,
        stamp_value: None,
    };
    let large = PropagationEntryRecord {
        transient_id: "c2".repeat(32),
        destination: "16".repeat(16),
        payload_hex: "16".repeat(40),
        received_at: 1_700_000_611,
        size_bytes: 40,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&small).expect("store small entry");
    daemon.store.upsert_propagation_entry(&large).expect("store large entry");
    for entry in [&small, &large] {
        daemon
            .store
            .mark_peer_unhandled_propagation("peer-transfer-budget", entry.transient_id.as_str())
            .expect("mark unhandled");
    }

    daemon
        .handle_rpc(rpc_request(59, "peer_sync", json!({ "peer": "peer-transfer-budget" })))
        .expect("budgeted peer sync");

    let handled = daemon
        .store
        .list_peer_handled_propagation_ids("peer-transfer-budget")
        .expect("handled ids");
    assert_eq!(handled, vec![small.transient_id]);
    let pending = daemon
        .store
        .list_peer_unhandled_propagation("peer-transfer-budget")
        .expect("pending propagation");
    assert_eq!(pending, vec![large]);
}

#[test]
fn peer_sync_leaves_entries_above_transfer_limit_unhandled_like_python() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(60, "peer_sync", json!({ "peer": "peer-transfer-oversize" })))
        .expect("initial peer sync");
    daemon.event_queue.lock().expect("event_queue mutex poisoned").clear();
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-transfer-oversize").expect("peer record");
        peer.propagation_transfer_limit = Some(80);
        peer.propagation_sync_limit = Some(1_000);
    }

    let oversized = PropagationEntryRecord {
        transient_id: "c3".repeat(32),
        destination: "16".repeat(16),
        payload_hex: "16".repeat(100),
        received_at: 1_700_000_612,
        size_bytes: 100,
        stamp_value: None,
    };
    let oversized_id = oversized.transient_id.clone();
    daemon.store.upsert_propagation_entry(&oversized).expect("store oversized entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-transfer-oversize", oversized.transient_id.as_str())
        .expect("mark unhandled");

    let result = daemon
        .handle_rpc(rpc_request(61, "peer_sync", json!({ "peer": "peer-transfer-oversize" })))
        .expect("peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["propagation"]["handled"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["offered"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["bytes"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["transfer_limited"].as_u64(), Some(1));
    assert_eq!(result["propagation"]["transfer_limited_bytes"].as_u64(), Some(100));
    assert_eq!(
        result["propagation"]["transfer_limited_ids"].as_array().expect("transfer limited ids"),
        &[json!(oversized_id.as_str())]
    );
    assert_eq!(result["messages"]["offered"].as_u64(), Some(1));
    assert_eq!(result["messages"]["unhandled"].as_u64(), Some(1));
    assert_eq!(result["sync_backoff"].as_u64(), Some(12 * 60));
    let last_sync_attempt = result["last_sync_attempt"].as_i64().expect("last sync attempt");
    assert_eq!(result["next_sync_attempt"].as_i64(), Some(last_sync_attempt + 12 * 60));

    let handled = daemon
        .store
        .list_peer_handled_propagation_ids("peer-transfer-oversize")
        .expect("handled ids");
    assert!(handled.is_empty());
    let pending = daemon
        .store
        .list_peer_unhandled_propagation("peer-transfer-oversize")
        .expect("pending propagation");
    assert_eq!(pending, vec![oversized]);

    let event = daemon
        .event_queue
        .lock()
        .expect("event_queue mutex poisoned")
        .iter()
        .rev()
        .find(|event| event.event_type == "peer_sync")
        .cloned()
        .expect("peer sync event");
    assert_eq!(event.payload["propagation"]["handled"].as_u64(), Some(0));
    assert_eq!(event.payload["propagation"]["offered"].as_u64(), Some(0));
    assert_eq!(event.payload["propagation"]["transfer_limited"].as_u64(), Some(1));
    assert_eq!(
        event.payload["propagation"]["transfer_limited_ids"]
            .as_array()
            .expect("event transfer limited ids"),
        &[json!(oversized_id.as_str())]
    );
    assert_eq!(event.payload["messages"]["offered"].as_u64(), Some(1));
    assert_eq!(event.payload["messages"]["unhandled"].as_u64(), Some(1));
    assert_eq!(event.payload["sync_backoff"].as_u64(), Some(12 * 60));
    assert_eq!(
        event.payload["next_sync_attempt"].as_i64(),
        Some(last_sync_attempt + 12 * 60)
    );
}

#[test]
fn peer_sync_applies_request_transfer_limit_without_persisting_it() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(60, "peer_sync", json!({ "peer": "peer-request-limit" })))
        .expect("initial peer sync");

    let oversized = PropagationEntryRecord {
        transient_id: "d4".repeat(32),
        destination: "16".repeat(16),
        payload_hex: "16".repeat(100),
        received_at: 1_700_000_613,
        size_bytes: 100,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&oversized).expect("store oversized entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-request-limit", oversized.transient_id.as_str())
        .expect("mark unhandled");

    let result = daemon
        .handle_rpc(rpc_request(
            61,
            "peer_sync",
            json!({
                "peer": "peer-request-limit",
                "transfer_limit_kb": 0.08,
            }),
        ))
        .expect("peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["propagation"]["handled"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["transfer_limited"].as_u64(), Some(1));
    assert_eq!(result["propagation"]["transfer_limit"].as_u64(), Some(80));

    let peers = daemon
        .handle_rpc(RpcRequest { id: 62, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-request-limit"))
        .expect("peer row");
    assert_eq!(row["propagation_transfer_limit"], JsonValue::Null);
}

#[test]
fn peer_sync_request_transfer_limit_does_not_loosen_peer_limit() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(60, "peer_sync", json!({ "peer": "peer-strict-limit" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-strict-limit").expect("peer record");
        peer.propagation_transfer_limit = Some(80);
        peer.propagation_sync_limit = Some(1_000);
    }

    let oversized = PropagationEntryRecord {
        transient_id: "d5".repeat(32),
        destination: "16".repeat(16),
        payload_hex: "16".repeat(100),
        received_at: 1_700_000_614,
        size_bytes: 100,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&oversized).expect("store oversized entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-strict-limit", oversized.transient_id.as_str())
        .expect("mark unhandled");

    let result = daemon
        .handle_rpc(rpc_request(
            61,
            "peer_sync",
            json!({
                "peer": "peer-strict-limit",
                "transfer_limit_kb": 1.0,
            }),
        ))
        .expect("peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["propagation"]["handled"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["transfer_limited"].as_u64(), Some(1));
    assert_eq!(result["propagation"]["transfer_limit"].as_u64(), Some(80));
}

#[test]
fn postponed_peer_sync_reports_request_transfer_limit() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(60, "peer_sync", json!({ "peer": "peer-postponed-limit" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-postponed-limit").expect("peer record");
        peer.next_sync_attempt = i64::MAX;
    }

    let result = daemon
        .handle_rpc(rpc_request(
            61,
            "peer_sync",
            json!({
                "peer": "peer-postponed-limit",
                "transfer_limit_kb": 0.08,
            }),
        ))
        .expect("postponed peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["synced"].as_bool(), Some(false));
    assert_eq!(result["postponed"].as_bool(), Some(true));
    assert_eq!(result["postpone_reason"].as_str(), Some("backoff"));
    assert_eq!(result["propagation"]["transfer_limit"].as_u64(), Some(80));
    assert_eq!(result["propagation"]["sync_limit"].as_u64(), Some(80));
    assert_eq!(result["transfer_limit"], JsonValue::Null);
}

#[test]
fn peer_sync_orders_offers_by_python_weight_before_sync_limit() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(62, "peer_sync", json!({ "peer": "peer-weight-order" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-weight-order").expect("peer record");
        peer.propagation_sync_limit = Some(152);
    }

    let older_large = PropagationEntryRecord {
        transient_id: "c4".repeat(32),
        destination: "16".repeat(16),
        payload_hex: "16".repeat(80),
        received_at: 1_700_000_612,
        size_bytes: 80,
        stamp_value: None,
    };
    let newer_small = PropagationEntryRecord {
        transient_id: "c5".repeat(32),
        destination: "16".repeat(16),
        payload_hex: "16".repeat(20),
        received_at: 1_700_000_613,
        size_bytes: 20,
        stamp_value: None,
    };
    for entry in [&older_large, &newer_small] {
        daemon.store.upsert_propagation_entry(entry).expect("store propagation entry");
        daemon
            .store
            .mark_peer_unhandled_propagation("peer-weight-order", entry.transient_id.as_str())
            .expect("mark unhandled");
    }

    let result = daemon
        .handle_rpc(rpc_request(63, "peer_sync", json!({ "peer": "peer-weight-order" })))
        .expect("peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(
        result["propagation"]["handled_ids"].as_array().expect("handled ids"),
        &[json!(newer_small.transient_id.as_str())]
    );
    assert_eq!(
        result["propagation"]["skipped_ids"].as_array().expect("skipped ids"),
        &[json!(older_large.transient_id.as_str())]
    );

    let handled = daemon
        .store
        .list_peer_handled_propagation_ids("peer-weight-order")
        .expect("handled ids");
    assert_eq!(handled, vec![newer_small.transient_id]);
    let pending = daemon
        .store
        .list_peer_unhandled_propagation("peer-weight-order")
        .expect("pending propagation");
    assert_eq!(pending, vec![older_large]);
}

#[test]
fn peer_sync_reports_propagation_transfer_accounting() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(60, "peer_sync", json!({ "peer": "peer-sync-report" })))
        .expect("initial peer sync");
    daemon.event_queue.lock().expect("event_queue mutex poisoned").clear();
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-sync-report").expect("peer record");
        peer.propagation_sync_limit = Some((24 + 20 + 32 + 16 + 1) as u32);
    }

    let small = PropagationEntryRecord {
        transient_id: "d1".repeat(32),
        destination: "17".repeat(16),
        payload_hex: "17".repeat(20),
        received_at: 1_700_000_612,
        size_bytes: 20,
        stamp_value: None,
    };
    let large = PropagationEntryRecord {
        transient_id: "d2".repeat(32),
        destination: "17".repeat(16),
        payload_hex: "17".repeat(100),
        received_at: 1_700_000_613,
        size_bytes: 100,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&small).expect("store small entry");
    daemon.store.upsert_propagation_entry(&large).expect("store large entry");
    for entry in [&small, &large] {
        daemon
            .store
            .mark_peer_unhandled_propagation("peer-sync-report", entry.transient_id.as_str())
            .expect("mark unhandled");
    }

    let result = daemon
        .handle_rpc(rpc_request(61, "peer_sync", json!({ "peer": "peer-sync-report" })))
        .expect("peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["propagation"]["handled"].as_u64(), Some(1));
    assert_eq!(result["propagation"]["skipped"].as_u64(), Some(1));
    assert_eq!(result["propagation"]["offered"].as_u64(), Some(2));
    assert_eq!(result["propagation"]["bytes"].as_u64(), Some(20));
    assert_eq!(result["propagation"]["offered_bytes"].as_u64(), Some(120));
    assert_eq!(result["propagation"]["remaining"].as_u64(), Some(1));
    assert_eq!(result["propagation"]["remaining_bytes"].as_u64(), Some(100));
    assert_eq!(
        result["propagation"]["sync_limit"].as_u64(),
        Some((24 + 20 + 32 + 16 + 1) as u64)
    );
    assert_eq!(result["acceptance_rate"].as_f64(), Some(0.5));
    assert_eq!(
        result["propagation"]["handled_ids"].as_array().expect("handled ids"),
        &[json!(small.transient_id.as_str())]
    );
    assert_eq!(
        result["propagation"]["skipped_ids"].as_array().expect("skipped ids"),
        &[json!(large.transient_id.as_str())]
    );

    let event = daemon
        .event_queue
        .lock()
        .expect("event_queue mutex poisoned")
        .iter()
        .rev()
        .find(|event| event.event_type == "peer_sync")
        .cloned()
        .expect("peer sync event");
    assert_eq!(event.payload["propagation"]["handled"].as_u64(), Some(1));
    assert_eq!(event.payload["propagation"]["skipped"].as_u64(), Some(1));
    assert_eq!(event.payload["propagation"]["offered"].as_u64(), Some(2));
    assert_eq!(event.payload["propagation"]["bytes"].as_u64(), Some(20));
    assert_eq!(event.payload["propagation"]["offered_bytes"].as_u64(), Some(120));
    assert_eq!(event.payload["propagation"]["remaining"].as_u64(), Some(1));
    assert_eq!(event.payload["propagation"]["remaining_bytes"].as_u64(), Some(100));
    assert_eq!(event.payload["synced"].as_bool(), Some(true));
    assert_eq!(event.payload["propagation"]["synced"].as_bool(), Some(true));
    assert_eq!(event.payload["propagation"]["postponed"].as_bool(), Some(false));
    assert_eq!(event.payload["acceptance_rate"].as_f64(), Some(0.5));
    assert_eq!(
        event.payload["propagation"]["handled_ids"].as_array().expect("event handled ids"),
        &[json!(small.transient_id.as_str())]
    );
    assert_eq!(
        event.payload["propagation"]["skipped_ids"].as_array().expect("event skipped ids"),
        &[json!(large.transient_id.as_str())]
    );

    let peers = daemon
        .handle_rpc(RpcRequest { id: 62, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-sync-report"))
        .expect("peer row");
    assert_eq!(row["tx_bytes"].as_u64(), Some(20));
    assert_eq!(row["acceptance_rate"].as_f64(), Some(0.5));
}

#[test]
fn peer_sync_updates_transfer_rate_from_transferred_bytes() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(63, "peer_sync", json!({ "peer": "peer-sync-rate" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-sync-rate").expect("peer record");
        peer.propagation_sync_limit = Some(1_000);
    }

    let entry = PropagationEntryRecord {
        transient_id: "d7".repeat(32),
        destination: "18".repeat(16),
        payload_hex: "25".repeat(40),
        received_at: 1_700_000_619,
        size_bytes: 40,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store propagation entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-sync-rate", entry.transient_id.as_str())
        .expect("mark unhandled");

    let result = daemon
        .handle_rpc(rpc_request(64, "peer_sync", json!({ "peer": "peer-sync-rate" })))
        .expect("peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["propagation"]["bytes"].as_u64(), Some(40));
    assert_eq!(result["sync_transfer_rate"].as_f64(), Some(40.0));
    assert_eq!(result["str"].as_u64(), Some(40));

    let peers = daemon
        .handle_rpc(RpcRequest { id: 65, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-sync-rate"))
        .expect("peer row");
    assert_eq!(row["sync_transfer_rate"].as_f64(), Some(40.0));
    assert_eq!(row["str"].as_u64(), Some(40));
}

#[test]
fn peer_sync_clears_transfer_rate_when_no_offers_remain() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(63, "peer_sync", json!({ "peer": "peer-sync-rate-empty" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-sync-rate-empty").expect("peer record");
        peer.propagation_sync_limit = Some(1_000);
    }

    let entry = PropagationEntryRecord {
        transient_id: "dc".repeat(32),
        destination: "18".repeat(16),
        payload_hex: "30".repeat(48),
        received_at: 1_700_000_624,
        size_bytes: 48,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store propagation entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-sync-rate-empty", entry.transient_id.as_str())
        .expect("mark unhandled");
    daemon
        .handle_rpc(rpc_request(64, "peer_sync", json!({ "peer": "peer-sync-rate-empty" })))
        .expect("peer sync with transfer");

    let result = daemon
        .handle_rpc(rpc_request(65, "peer_sync", json!({ "peer": "peer-sync-rate-empty" })))
        .expect("peer sync without offers")
        .result
        .expect("peer sync result");
    assert_eq!(result["propagation"]["offered"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["bytes"].as_u64(), Some(0));
    assert_eq!(result["sync_transfer_rate"].as_f64(), Some(0.0));
    assert_eq!(result["str"].as_u64(), Some(0));

    let peers = daemon
        .handle_rpc(RpcRequest { id: 66, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-sync-rate-empty"))
        .expect("peer row");
    assert_eq!(row["sync_transfer_rate"].as_f64(), Some(0.0));
    assert_eq!(row["str"].as_u64(), Some(0));
}

#[test]
fn peer_sync_clears_transfer_rate_when_offers_are_skipped() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(63, "peer_sync", json!({ "peer": "peer-sync-rate-skipped" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-sync-rate-skipped").expect("peer record");
        peer.propagation_sync_limit = Some(1_000);
    }

    let handled = PropagationEntryRecord {
        transient_id: "d8".repeat(32),
        destination: "18".repeat(16),
        payload_hex: "26".repeat(40),
        received_at: 1_700_000_620,
        size_bytes: 40,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&handled).expect("store handled entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-sync-rate-skipped", handled.transient_id.as_str())
        .expect("mark handled unhandled");
    daemon
        .handle_rpc(rpc_request(64, "peer_sync", json!({ "peer": "peer-sync-rate-skipped" })))
        .expect("peer sync with transfer");

    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-sync-rate-skipped").expect("peer record");
        peer.propagation_sync_limit = Some(24 + 40 + 16);
    }
    let skipped = PropagationEntryRecord {
        transient_id: "d9".repeat(32),
        destination: "18".repeat(16),
        payload_hex: "27".repeat(40),
        received_at: 1_700_000_621,
        size_bytes: 40,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&skipped).expect("store skipped entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-sync-rate-skipped", skipped.transient_id.as_str())
        .expect("mark skipped unhandled");

    let result = daemon
        .handle_rpc(rpc_request(65, "peer_sync", json!({ "peer": "peer-sync-rate-skipped" })))
        .expect("peer sync with skipped offer")
        .result
        .expect("peer sync result");
    assert_eq!(result["propagation"]["handled"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["skipped"].as_u64(), Some(1));
    assert_eq!(result["sync_transfer_rate"].as_f64(), Some(0.0));
    assert_eq!(result["str"].as_u64(), Some(0));

    let peers = daemon
        .handle_rpc(RpcRequest { id: 66, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-sync-rate-skipped"))
        .expect("peer row");
    assert_eq!(row["sync_transfer_rate"].as_f64(), Some(0.0));
    assert_eq!(row["str"].as_u64(), Some(0));

    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-sync-rate-skipped").expect("peer record");
        peer.propagation_transfer_limit = None;
        peer.propagation_sync_limit = Some(1_000);
        peer.next_sync_attempt = 0;
        peer.sync_backoff = 0;
    }
    let second_handled = PropagationEntryRecord {
        transient_id: "da".repeat(32),
        destination: "18".repeat(16),
        payload_hex: "28".repeat(32),
        received_at: 1_700_000_622,
        size_bytes: 32,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&second_handled).expect("store second handled entry");
    daemon
        .store
        .mark_peer_unhandled_propagation(
            "peer-sync-rate-skipped",
            second_handled.transient_id.as_str(),
        )
        .expect("mark second handled unhandled");
    daemon
        .handle_rpc(rpc_request(67, "peer_sync", json!({ "peer": "peer-sync-rate-skipped" })))
        .expect("peer sync with second transfer");

    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-sync-rate-skipped").expect("peer record");
        peer.propagation_transfer_limit = Some(80);
        peer.propagation_sync_limit = Some(1_000);
    }
    let transfer_limited = PropagationEntryRecord {
        transient_id: "db".repeat(32),
        destination: "18".repeat(16),
        payload_hex: "29".repeat(100),
        received_at: 1_700_000_623,
        size_bytes: 100,
        stamp_value: None,
    };
    daemon
        .store
        .upsert_propagation_entry(&transfer_limited)
        .expect("store transfer limited entry");
    daemon
        .store
        .mark_peer_unhandled_propagation(
            "peer-sync-rate-skipped",
            transfer_limited.transient_id.as_str(),
        )
        .expect("mark transfer limited unhandled");

    let result = daemon
        .handle_rpc(rpc_request(68, "peer_sync", json!({ "peer": "peer-sync-rate-skipped" })))
        .expect("peer sync with transfer-limited offer")
        .result
        .expect("peer sync result");
    assert_eq!(result["propagation"]["handled"].as_u64(), Some(0));
    assert_eq!(result["propagation"]["transfer_limited"].as_u64(), Some(1));
    assert_eq!(result["sync_transfer_rate"].as_f64(), Some(0.0));
    assert_eq!(result["str"].as_u64(), Some(0));
}

#[test]
fn peer_sync_reports_transferred_propagation_messages() {
    let store = MessagesStore::in_memory().expect("store");
    let daemon = RpcDaemon::with_store(store, hex::encode([2u8; 16]));
    let peer_id = hex::encode([3u8; 16]);
    daemon
        .handle_rpc(rpc_request(63, "peer_sync", json!({ "peer": peer_id })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut(peer_id.as_str()).expect("peer record");
        peer.propagation_stamp_cost = Some(1);
        peer.propagation_stamp_cost_flexibility = Some(1);
        peer.peering_cost = Some(1);
    }

    let entry = PropagationEntryRecord {
        transient_id: "d3".repeat(32),
        destination: "18".repeat(16),
        payload_hex: "21".repeat(24),
        received_at: 1_700_000_614,
        size_bytes: 24,
        stamp_value: Some(11),
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store propagation entry");
    daemon
        .store
        .mark_peer_unhandled_propagation(peer_id.as_str(), entry.transient_id.as_str())
        .expect("mark unhandled");

    let result = daemon
        .handle_rpc(rpc_request(64, "peer_sync", json!({ "peer": peer_id })))
        .expect("peer sync")
        .result
        .expect("peer sync result");
    let messages = result["propagation"]["messages"].as_array().expect("propagation messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["transient_id"].as_str(), Some(entry.transient_id.as_str()));
    assert_eq!(messages[0]["destination"].as_str(), Some(entry.destination.as_str()));
    assert_eq!(messages[0]["payload_hex"].as_str(), Some(entry.payload_hex.as_str()));
    assert_eq!(messages[0]["received_at"].as_i64(), Some(entry.received_at));
    assert_eq!(messages[0]["size_bytes"].as_u64(), Some(entry.size_bytes));
    assert_eq!(messages[0]["stamp_value"].as_u64(), Some(11));
}

#[test]
fn peer_sync_drops_low_value_stamped_entries_before_offer() {
    let store = MessagesStore::in_memory().expect("store");
    let daemon = RpcDaemon::with_store(store, hex::encode([2u8; 16]));
    let peer_id = hex::encode([5u8; 16]);
    daemon
        .handle_rpc(rpc_request(63, "peer_sync", json!({ "peer": peer_id })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut(peer_id.as_str()).expect("peer record");
        peer.propagation_sync_limit = Some(1_000);
        peer.propagation_stamp_cost = Some(8);
        peer.propagation_stamp_cost_flexibility = Some(2);
        peer.peering_cost = Some(1);
    }

    let low_value = PropagationEntryRecord {
        transient_id: "d5".repeat(32),
        destination: "18".repeat(16),
        payload_hex: "23".repeat(24),
        received_at: 1_700_000_617,
        size_bytes: 24,
        stamp_value: Some(5),
    };
    let accepted = PropagationEntryRecord {
        transient_id: "d6".repeat(32),
        destination: "18".repeat(16),
        payload_hex: "24".repeat(24),
        received_at: 1_700_000_618,
        size_bytes: 24,
        stamp_value: Some(6),
    };
    for entry in [&low_value, &accepted] {
        daemon.store.upsert_propagation_entry(entry).expect("store propagation entry");
        daemon
            .store
            .mark_peer_unhandled_propagation(peer_id.as_str(), entry.transient_id.as_str())
            .expect("mark unhandled");
    }

    let result = daemon
        .handle_rpc(rpc_request(64, "peer_sync", json!({ "peer": peer_id })))
        .expect("peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["propagation"]["handled"].as_u64(), Some(1));
    assert_eq!(result["propagation"]["skipped"].as_u64(), Some(0));
    assert_eq!(
        result["propagation"]["handled_ids"].as_array().expect("handled ids"),
        &[json!(accepted.transient_id.as_str())]
    );

    let pending = daemon
        .store
        .list_peer_unhandled_propagation(peer_id.as_str())
        .expect("pending propagation");
    assert!(pending.is_empty());
    let handled = daemon
        .store
        .list_peer_handled_propagation_ids(peer_id.as_str())
        .expect("handled propagation");
    assert_eq!(handled, vec![accepted.transient_id]);
}

#[test]
fn peer_sync_result_and_event_report_message_accounting() {
    let store = MessagesStore::in_memory().expect("store");
    let daemon = RpcDaemon::with_store(store, hex::encode([2u8; 16]));
    let peer_id = hex::encode([4u8; 16]);
    daemon
        .handle_rpc(rpc_request(63, "peer_sync", json!({ "peer": peer_id })))
        .expect("initial peer sync");
    daemon.event_queue.lock().expect("event_queue mutex poisoned").clear();
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut(peer_id.as_str()).expect("peer record");
        peer.propagation_stamp_cost = Some(1);
        peer.propagation_stamp_cost_flexibility = Some(1);
        peer.peering_cost = Some(1);
    }

    let entry = PropagationEntryRecord {
        transient_id: "d4".repeat(32),
        destination: "18".repeat(16),
        payload_hex: "22".repeat(24),
        received_at: 1_700_000_616,
        size_bytes: 24,
        stamp_value: Some(12),
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store propagation entry");
    daemon
        .store
        .mark_peer_unhandled_propagation(peer_id.as_str(), entry.transient_id.as_str())
        .expect("mark unhandled");

    let result = daemon
        .handle_rpc(rpc_request(64, "peer_sync", json!({ "peer": peer_id })))
        .expect("peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["messages"]["offered"].as_u64(), Some(1));
    assert_eq!(result["messages"]["unhandled"].as_u64(), Some(0));
    assert_eq!(result["messages"]["offered_bytes"].as_u64(), Some(24));
    assert_eq!(result["messages"]["unhandled_bytes"].as_u64(), Some(0));
    assert_eq!(
        result["messages"]["handled_ids"].as_array().expect("result handled ids"),
        &[json!(entry.transient_id.as_str())]
    );
    assert_eq!(
        result["messages"]["unhandled_ids"].as_array().expect("result unhandled ids"),
        &[] as &[JsonValue]
    );

    let event = daemon
        .event_queue
        .lock()
        .expect("event_queue mutex poisoned")
        .iter()
        .rev()
        .find(|event| event.event_type == "peer_sync")
        .cloned()
        .expect("peer sync event");
    assert_eq!(event.payload["messages"]["offered"].as_u64(), Some(1));
    assert_eq!(event.payload["messages"]["unhandled"].as_u64(), Some(0));
    assert_eq!(event.payload["messages"]["offered_bytes"].as_u64(), Some(24));
    assert_eq!(event.payload["messages"]["unhandled_bytes"].as_u64(), Some(0));
    assert_eq!(
        event.payload["messages"]["handled_ids"].as_array().expect("event handled ids"),
        &[json!(entry.transient_id.as_str())]
    );
    assert_eq!(
        event.payload["messages"]["unhandled_ids"].as_array().expect("event unhandled ids"),
        &[] as &[JsonValue]
    );
}

#[test]
fn peer_sync_result_and_event_report_transfer_and_stamp_policy() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(65, "peer_sync", json!({ "peer": "peer-sync-policy" })))
        .expect("initial peer sync");
    daemon.event_queue.lock().expect("event_queue mutex poisoned").clear();

    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-sync-policy").expect("peer record");
        peer.peer_type = Some("static".to_string());
        peer.propagation_transfer_limit = Some(333);
        peer.propagation_sync_limit = Some(999);
        peer.propagation_stamp_cost = Some(8);
        peer.propagation_stamp_cost_flexibility = Some(2);
        peer.sync_transfer_rate = 12_345.0;
        peer.peering_timebase = 1_700_000_123;
        peer.network_distance = 4;
        peer.rx_bytes = 55;
        peer.tx_bytes = 77;
    }

    let result = daemon
        .handle_rpc(rpc_request(66, "peer_sync", json!({ "peer": "peer-sync-policy" })))
        .expect("peer sync")
        .result
        .expect("peer sync result");
    assert_eq!(result["peer_type"].as_str(), Some("static"));
    assert_eq!(result["state"].as_u64(), Some(0));
    assert_eq!(result["sync_strategy"].as_u64(), Some(2));
    assert_eq!(result["ler"].as_u64(), Some(0));
    assert_eq!(result["network_distance"].as_u64(), Some(4));
    assert_eq!(result["peering_timebase"].as_i64(), Some(1_700_000_123));
    assert_eq!(result["rx_bytes"].as_u64(), Some(55));
    assert_eq!(result["tx_bytes"].as_u64(), Some(77));
    assert_eq!(result["propagation_transfer_limit"].as_u64(), Some(333));
    assert_eq!(result["propagation_sync_limit"].as_u64(), Some(999));
    assert_eq!(result["propagation_stamp_cost"].as_u64(), Some(8));
    assert_eq!(result["propagation_stamp_cost_flexibility"].as_u64(), Some(2));
    assert_eq!(result["transfer_limit"].as_u64(), Some(333));
    assert_eq!(result["sync_limit"].as_u64(), Some(999));
    assert_eq!(result["propagation"]["transfer_limit"].as_u64(), Some(333));
    assert_eq!(result["propagation"]["sync_limit"].as_u64(), Some(999));
    assert_eq!(result["propagation"]["target_stamp_cost"].as_u64(), Some(8));
    assert_eq!(result["propagation"]["stamp_cost_flexibility"].as_u64(), Some(2));
    assert_eq!(result["target_stamp_cost"].as_u64(), Some(8));
    assert_eq!(result["stamp_cost_flexibility"].as_u64(), Some(2));
    assert_eq!(result["sync_transfer_rate"].as_f64(), Some(0.0));
    assert_eq!(result["str"].as_u64(), Some(0));

    let event = daemon
        .event_queue
        .lock()
        .expect("event_queue mutex poisoned")
        .iter()
        .rev()
        .find(|event| event.event_type == "peer_sync")
        .cloned()
        .expect("peer sync event");
    assert_eq!(event.payload["peer_type"].as_str(), Some("static"));
    assert_eq!(event.payload["state"].as_u64(), Some(0));
    assert_eq!(event.payload["sync_strategy"].as_u64(), Some(2));
    assert_eq!(event.payload["ler"].as_u64(), Some(0));
    assert_eq!(event.payload["network_distance"].as_u64(), Some(4));
    assert_eq!(event.payload["peering_timebase"].as_i64(), Some(1_700_000_123));
    assert_eq!(event.payload["rx_bytes"].as_u64(), Some(55));
    assert_eq!(event.payload["tx_bytes"].as_u64(), Some(77));
    assert_eq!(
        event.payload["propagation_transfer_limit"].as_u64(),
        Some(333)
    );
    assert_eq!(event.payload["propagation_sync_limit"].as_u64(), Some(999));
    assert_eq!(event.payload["propagation_stamp_cost"].as_u64(), Some(8));
    assert_eq!(
        event.payload["propagation_stamp_cost_flexibility"].as_u64(),
        Some(2)
    );
    assert_eq!(event.payload["transfer_limit"].as_u64(), Some(333));
    assert_eq!(event.payload["sync_limit"].as_u64(), Some(999));
    assert_eq!(event.payload["propagation"]["transfer_limit"].as_u64(), Some(333));
    assert_eq!(event.payload["propagation"]["sync_limit"].as_u64(), Some(999));
    assert_eq!(event.payload["propagation"]["target_stamp_cost"].as_u64(), Some(8));
    assert_eq!(
        event.payload["propagation"]["stamp_cost_flexibility"].as_u64(),
        Some(2)
    );
    assert_eq!(event.payload["target_stamp_cost"].as_u64(), Some(8));
    assert_eq!(event.payload["stamp_cost_flexibility"].as_u64(), Some(2));
    assert_eq!(event.payload["sync_transfer_rate"].as_f64(), Some(0.0));
    assert_eq!(event.payload["str"].as_u64(), Some(0));
}

#[test]
fn list_peers_includes_propagation_marks_in_message_counters() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(56, "peer_sync", json!({ "peer": "peer-propagation-stats" })))
        .expect("peer sync");
    let handled = PropagationEntryRecord {
        transient_id: "ac".repeat(32),
        destination: "13".repeat(16),
        payload_hex: "13".repeat(16),
        received_at: 1_700_000_606,
        size_bytes: 16,
        stamp_value: None,
    };
    let unhandled = PropagationEntryRecord {
        transient_id: "ad".repeat(32),
        destination: "14".repeat(16),
        payload_hex: "14".repeat(24),
        received_at: 1_700_000_607,
        size_bytes: 24,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&handled).expect("store handled entry");
    daemon.store.upsert_propagation_entry(&unhandled).expect("store unhandled entry");
    daemon
        .store
        .mark_peer_handled_propagation("peer-propagation-stats", handled.transient_id.as_str())
        .expect("mark handled");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-propagation-stats", unhandled.transient_id.as_str())
        .expect("mark unhandled");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 57, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-propagation-stats"))
        .expect("peer row");
    assert_eq!(row["messages"]["offered"].as_u64(), Some(2));
    assert_eq!(row["messages"]["unhandled"].as_u64(), Some(1));
    assert_eq!(row["messages"]["offered_bytes"].as_u64(), Some(40));
    assert_eq!(row["messages"]["unhandled_bytes"].as_u64(), Some(24));
    assert_eq!(
        row["handled_ids"].as_array().expect("handled ids"),
        &[json!(handled.transient_id.as_str())]
    );
    assert_eq!(
        row["unhandled_ids"].as_array().expect("unhandled ids"),
        &[json!(unhandled.transient_id.as_str())]
    );
    assert_eq!(
        row["messages"]["handled_ids"].as_array().expect("message handled ids"),
        &[json!(handled.transient_id.as_str())]
    );
    assert_eq!(
        row["messages"]["unhandled_ids"].as_array().expect("message unhandled ids"),
        &[json!(unhandled.transient_id.as_str())]
    );
}

#[test]
fn list_peers_exposes_peering_key_value_when_cost_is_known() {
    let store = MessagesStore::in_memory().expect("store");
    let daemon = RpcDaemon::with_store(store, hex::encode([2u8; 16]));
    let peer = hex::encode([3u8; 16]);

    daemon
        .accept_announce_with_metadata(
            peer.clone(),
            1_700_000_610,
            None,
            None,
            None,
            Some(vec!["propagation".to_string()]),
            None,
            None,
            None,
            Some(1),
            Some(Some(1)),
            Some(Some(1)),
            None,
            Some(1),
            None,
            None,
            None,
            None,
        )
        .expect("accept propagation peer announce");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 55, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some(peer.as_str()))
        .expect("peer row");
    assert_eq!(row["peering_cost"].as_u64(), Some(1));
    assert!(row["peering_key"].as_u64().is_some_and(|value| value >= 1));
}

#[test]
fn peer_sync_result_and_event_expose_peering_key_value_when_cost_is_known() {
    let store = MessagesStore::in_memory().expect("store");
    let daemon = RpcDaemon::with_store(store, hex::encode([2u8; 16]));
    let peer = hex::encode([3u8; 16]);

    daemon
        .accept_announce_with_metadata(
            peer.clone(),
            1_700_000_611,
            None,
            None,
            None,
            Some(vec!["propagation".to_string()]),
            None,
            None,
            None,
            Some(1),
            Some(Some(1)),
            Some(Some(1)),
            None,
            Some(1),
            None,
            None,
            None,
            None,
        )
        .expect("accept propagation peer announce");
    daemon.event_queue.lock().expect("event_queue mutex poisoned").clear();

    let result = daemon
        .handle_rpc(rpc_request(56, "peer_sync", json!({ "peer": peer })))
        .expect("peer sync")
        .result
        .expect("peer sync result");
    let peering_key = result["peering_key"].as_u64().expect("peering key");
    assert!(peering_key >= 1);
    assert_eq!(result["propagation"]["peering_key"].as_u64(), Some(peering_key));

    let event = daemon
        .event_queue
        .lock()
        .expect("event_queue mutex poisoned")
        .iter()
        .rev()
        .find(|event| event.event_type == "peer_sync")
        .cloned()
        .expect("peer sync event");
    assert_eq!(event.payload["peering_key"].as_u64(), Some(peering_key));
    assert_eq!(event.payload["propagation"]["peering_key"].as_u64(), Some(peering_key));
}

#[test]
fn peer_sync_preserves_existing_auto_peer_type() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            52,
            "propagation_enable",
            json!({
                "enabled": true,
                "autopeer": true,
            }),
        ))
        .expect("enable propagation");

    daemon
        .accept_announce_with_metadata(
            "peer-auto".to_string(),
            1_700_000_220,
            Some("Peer Auto".to_string()),
            Some("announce".to_string()),
            None,
            Some(vec!["propagation".to_string()]),
            None,
            None,
            None,
            Some(4),
            Some(Some(1)),
            Some(Some(4)),
            None,
            Some(1),
            None,
            None,
            None,
            None,
        )
        .expect("accept announce");

    daemon
        .handle_rpc(rpc_request(53, "peer_sync", json!({ "peer": "peer-auto" })))
        .expect("peer sync");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 54, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"].as_array().and_then(|rows| rows.first()).expect("peer row");
    assert_eq!(row["peer_type"].as_str(), Some("auto"));
}

#[test]
fn peer_sync_reports_python_status_type_alias() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            55,
            "propagation_enable",
            json!({
                "enabled": true,
                "static_peers": ["peer-static-alias"],
            }),
        ))
        .expect("enable propagation");
    daemon.event_queue.lock().expect("event_queue mutex poisoned").clear();

    let static_result = daemon
        .handle_rpc(rpc_request(56, "peer_sync", json!({ "peer": "peer-static-alias" })))
        .expect("static peer sync")
        .result
        .expect("static peer sync result");
    assert_eq!(static_result["peer_type"].as_str(), Some("static"));
    assert_eq!(static_result["type"].as_str(), Some("static"));

    let static_event = daemon
        .event_queue
        .lock()
        .expect("event_queue mutex poisoned")
        .iter()
        .rev()
        .find(|event| event.event_type == "peer_sync")
        .cloned()
        .expect("static peer sync event");
    assert_eq!(static_event.payload["peer_type"].as_str(), Some("static"));
    assert_eq!(static_event.payload["type"].as_str(), Some("static"));
    daemon.event_queue.lock().expect("event_queue mutex poisoned").clear();

    let manual_result = daemon
        .handle_rpc(rpc_request(57, "peer_sync", json!({ "peer": "peer-manual-alias" })))
        .expect("manual peer sync")
        .result
        .expect("manual peer sync result");
    assert_eq!(manual_result["peer_type"].as_str(), Some("manual"));
    assert_eq!(manual_result["type"].as_str(), Some("discovered"));

    let manual_event = daemon
        .event_queue
        .lock()
        .expect("event_queue mutex poisoned")
        .iter()
        .rev()
        .find(|event| event.event_type == "peer_sync")
        .cloned()
        .expect("manual peer sync event");
    assert_eq!(manual_event.payload["peer_type"].as_str(), Some("manual"));
    assert_eq!(manual_event.payload["type"].as_str(), Some("discovered"));
}

#[test]
fn stale_high_cost_announce_does_not_remove_newer_autopeer() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            55,
            "propagation_enable",
            json!({
                "enabled": true,
                "autopeer": true,
                "remote_peering_cost_max": 5,
            }),
        ))
        .expect("enable propagation");

    daemon
        .accept_announce_with_metadata(
            "peer-auto".to_string(),
            1_700_000_400,
            None,
            None,
            None,
            Some(vec!["propagation".to_string()]),
            None,
            None,
            None,
            Some(3),
            Some(Some(1)),
            Some(Some(4)),
            None,
            Some(1),
            None,
            None,
            None,
            None,
        )
        .expect("accept initial announce");

    daemon
        .accept_announce_with_metadata(
            "peer-auto".to_string(),
            1_700_000_399,
            None,
            None,
            None,
            Some(vec!["propagation".to_string()]),
            None,
            None,
            None,
            Some(3),
            Some(Some(1)),
            Some(Some(9)),
            None,
            Some(1),
            None,
            None,
            None,
            None,
        )
        .expect("accept stale high-cost announce");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 56, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"].as_array().and_then(|rows| rows.first()).expect("peer row");
    assert_eq!(row["peer"].as_str(), Some("peer-auto"));
    assert_eq!(row["peer_type"].as_str(), Some("auto"));
}

#[test]
fn high_cost_announce_does_not_remove_manual_peer() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            57,
            "propagation_enable",
            json!({
                "enabled": true,
                "autopeer": true,
                "remote_peering_cost_max": 5,
            }),
        ))
        .expect("enable propagation");

    daemon
        .handle_rpc(rpc_request(58, "peer_sync", json!({ "peer": "peer-manual" })))
        .expect("manual peer sync");

    daemon
        .accept_announce_with_metadata(
            "peer-manual".to_string(),
            1_700_000_500,
            None,
            None,
            None,
            Some(vec!["propagation".to_string()]),
            None,
            None,
            None,
            Some(3),
            Some(Some(1)),
            Some(Some(9)),
            None,
            Some(1),
            None,
            None,
            None,
            None,
        )
        .expect("accept high-cost announce");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 59, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"].as_array().and_then(|rows| rows.first()).expect("peer row");
    assert_eq!(row["peer"].as_str(), Some("peer-manual"));
    assert_eq!(row["peer_type"].as_str(), Some("manual"));
}

include!("status_snapshot_propagation_ingest.rs");

struct TestRemoteControlBridge {
    result: Result<JsonValue, std::io::ErrorKind>,
}

impl RemoteControlBridge for TestRemoteControlBridge {
    fn propagation_remote_status(
        &self,
        _remote: &str,
        _identity_private_key_hex: Option<&str>,
        _timeout_secs: f64,
    ) -> Result<JsonValue, std::io::Error> {
        Ok(json!({"status": "ok"}))
    }

    fn propagation_remote_sync(
        &self,
        remote: &str,
        peer: &str,
        _identity_private_key_hex: Option<&str>,
        _timeout_secs: f64,
        transfer_limit_kb: Option<f64>,
    ) -> Result<JsonValue, std::io::Error> {
        assert_eq!(transfer_limit_kb, None);
        self.result.clone().map(|mut result| {
            result["remote"] = json!(remote);
            result["peer"] = json!(peer);
            result
        }).map_err(|kind| std::io::Error::new(kind, "remote sync failed"))
    }

    fn propagation_remote_download(
        &self,
        remote: &str,
        _identity_private_key_hex: Option<&str>,
        _timeout_secs: f64,
        transfer_limit_kb: Option<f64>,
    ) -> Result<JsonValue, std::io::Error> {
        assert_eq!(transfer_limit_kb, None);
        self.result.clone().map(|mut result| {
            result["remote"] = json!(remote);
            result
        }).map_err(|kind| std::io::Error::new(kind, "remote download failed"))
    }

    fn propagation_remote_unpeer(
        &self,
        remote: &str,
        peer: &str,
        _identity_private_key_hex: Option<&str>,
        _timeout_secs: f64,
    ) -> Result<JsonValue, std::io::Error> {
        self.result
            .clone()
            .map(|mut result| {
                result["remote"] = json!(remote);
                result["peer"] = json!(peer);
                result["unpeered"] = json!(true);
                result
            })
            .map_err(|kind| std::io::Error::new(kind, "remote unpeer failed"))
    }

    fn propagation_remote_fetch(
        &self,
        _remote: &str,
        _identity_private_key_hex: Option<&str>,
        _timeout_secs: f64,
        _transfer_limit_kb: Option<f64>,
    ) -> Result<JsonValue, std::io::Error> {
        self.result.clone().map_err(|kind| std::io::Error::new(kind, "remote fetch failed"))
    }
}

struct TransferLimitRemoteControlBridge;

impl RemoteControlBridge for TransferLimitRemoteControlBridge {
    fn propagation_remote_status(
        &self,
        _remote: &str,
        _identity_private_key_hex: Option<&str>,
        _timeout_secs: f64,
    ) -> Result<JsonValue, std::io::Error> {
        Ok(json!({"status": "ok"}))
    }

    fn propagation_remote_sync(
        &self,
        _remote: &str,
        _peer: &str,
        _identity_private_key_hex: Option<&str>,
        _timeout_secs: f64,
        transfer_limit_kb: Option<f64>,
    ) -> Result<JsonValue, std::io::Error> {
        assert_eq!(transfer_limit_kb, Some(42.5));
        Ok(json!({"synced": true}))
    }

    fn propagation_remote_download(
        &self,
        _remote: &str,
        _identity_private_key_hex: Option<&str>,
        _timeout_secs: f64,
        transfer_limit_kb: Option<f64>,
    ) -> Result<JsonValue, std::io::Error> {
        assert_eq!(transfer_limit_kb, Some(42.5));
        Ok(json!({
            "downloaded_count": 0,
            "messages": [],
        }))
    }

    fn propagation_remote_unpeer(
        &self,
        _remote: &str,
        _peer: &str,
        _identity_private_key_hex: Option<&str>,
        _timeout_secs: f64,
    ) -> Result<JsonValue, std::io::Error> {
        Ok(json!({"unpeered": true}))
    }

    fn propagation_remote_fetch(
        &self,
        _remote: &str,
        _identity_private_key_hex: Option<&str>,
        _timeout_secs: f64,
        _transfer_limit_kb: Option<f64>,
    ) -> Result<JsonValue, std::io::Error> {
        Ok(json!({"messages": []}))
    }
}

#[test]
fn selected_propagation_node_updates_status_snapshot() {
    let daemon = RpcDaemon::test_instance();

    daemon
        .handle_rpc(rpc_request(
            67,
            "set_outbound_propagation_node",
            json!({
                "peer": "  peer-propagation-node  ",
            }),
        ))
        .expect("set propagation node");

    let propagation_status = daemon
        .handle_rpc(RpcRequest { id: 68, method: "propagation_status".to_string(), params: None })
        .expect("propagation status")
        .result
        .expect("propagation status result");
    assert_eq!(
        propagation_status["propagation"]["selected_node"].as_str(),
        Some("peer-propagation-node")
    );

    let daemon_status = daemon
        .handle_rpc(RpcRequest { id: 69, method: "daemon_status_ex".to_string(), params: None })
        .expect("daemon status")
        .result
        .expect("daemon status result");
    assert_eq!(
        daemon_status["propagation"]["selected_node"].as_str(),
        Some("peer-propagation-node")
    );

    let nodes = daemon
        .handle_rpc(RpcRequest {
            id: 72,
            method: "list_propagation_nodes".to_string(),
            params: None,
        })
        .expect("list propagation nodes")
        .result
        .expect("list propagation nodes result");
    let node = nodes["nodes"].as_array().and_then(|rows| rows.first()).expect("node row");
    assert_eq!(node["peer"].as_str(), Some("peer-propagation-node"));
    assert_eq!(node["selected"].as_bool(), Some(true));
    assert_eq!(node["capabilities"], json!(["propagation"]));

    daemon
        .handle_rpc(rpc_request(70, "set_outbound_propagation_node", json!({ "peer": " " })))
        .expect("clear propagation node");
    let cleared = daemon
        .handle_rpc(RpcRequest { id: 71, method: "propagation_status".to_string(), params: None })
        .expect("propagation status")
        .result
        .expect("propagation status result");
    assert_eq!(cleared["propagation"]["selected_node"], JsonValue::Null);
}

#[test]
fn propagation_remote_sync_updates_lifecycle_status() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({"synced": true})),
    }));

    let result = daemon
        .handle_rpc(rpc_request(
            72,
            "propagation_remote_sync",
            json!({
                "remote": "remote-node",
                "peer": "peer-a",
            }),
        ))
        .expect("remote sync")
        .result
        .expect("remote sync result");
    let result_propagation = &result["propagation"];
    assert_eq!(result_propagation["sync_state"].as_u64(), Some(0x07));
    assert_eq!(result_propagation["state_name"].as_str(), Some("completed"));
    assert_eq!(result_propagation["sync_progress"].as_f64(), Some(1.0));
    assert!(result_propagation["last_sync_started"].as_i64().is_some());
    assert!(result_propagation["last_sync_completed"].as_i64().is_some());
    assert_eq!(result_propagation["last_sync_error"], JsonValue::Null);

    let status = daemon
        .handle_rpc(RpcRequest { id: 73, method: "propagation_status".to_string(), params: None })
        .expect("propagation status")
        .result
        .expect("propagation status result");
    let propagation = &status["propagation"];
    assert_eq!(propagation["sync_state"].as_u64(), Some(0x07));
    assert_eq!(propagation["state_name"].as_str(), Some("completed"));
    assert_eq!(propagation["sync_progress"].as_f64(), Some(1.0));
    assert!(propagation["last_sync_started"].as_i64().is_some());
    assert!(propagation["last_sync_completed"].as_i64().is_some());
    assert_eq!(propagation["last_sync_error"], JsonValue::Null);
}

#[test]
fn propagation_remote_sync_updates_peer_runtime_state() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({"synced": true})),
    }));
    daemon
        .handle_rpc(rpc_request(73, "peer_sync", json!({ "peer": "peer-remote-sync-state" })))
        .expect("initial peer sync");
    daemon.event_queue.lock().expect("event_queue mutex poisoned").clear();
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-remote-sync-state").expect("peer record");
        peer.alive = false;
        peer.sync_backoff = 12 * 60;
        peer.next_sync_attempt = 1_700_010_000;
        peer.acceptance_rate = 0.25;
    }

    daemon
        .handle_rpc(rpc_request(
            74,
            "propagation_remote_sync",
            json!({
                "remote": "remote-node",
                "peer": "peer-remote-sync-state",
            }),
        ))
        .expect("remote sync");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 75, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-remote-sync-state"))
        .expect("peer row");
    assert_eq!(row["alive"].as_bool(), Some(true));
    assert!(row["last_sync_attempt"].as_i64().is_some_and(|value| value > 0));
    assert_eq!(row["sync_backoff"].as_u64(), Some(0));
    assert_eq!(row["next_sync_attempt"].as_i64(), Some(0));
    assert!(row["acceptance_rate"].as_f64().is_some_and(|value| value > 0.25));

    let event = daemon
        .event_queue
        .lock()
        .expect("event_queue mutex poisoned")
        .iter()
        .rev()
        .find(|event| event.event_type == "peer_sync")
        .cloned()
        .expect("peer sync event");
    assert_eq!(event.payload["peer"].as_str(), Some("peer-remote-sync-state"));
    assert_eq!(event.payload["remote"].as_str(), Some("remote-node"));
    assert_eq!(event.payload["remote_sync"].as_bool(), Some(true));
    assert_eq!(event.payload["synced"].as_bool(), Some(true));
    assert_eq!(event.payload["alive"].as_bool(), Some(true));
    assert_eq!(event.payload["sync_backoff"].as_u64(), Some(0));
    assert_eq!(event.payload["next_sync_attempt"].as_i64(), Some(0));
}

#[test]
fn propagation_remote_sync_creates_missing_peer_record() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({"synced": true})),
    }));

    daemon
        .handle_rpc(rpc_request(
            76,
            "propagation_remote_sync",
            json!({
                "remote": "remote-node",
                "peer": "peer-remote-sync-created",
            }),
        ))
        .expect("remote sync");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 77, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-remote-sync-created"))
        .expect("peer row");
    assert_eq!(row["peer_type"].as_str(), Some("manual"));
    assert_eq!(row["alive"].as_bool(), Some(true));
    assert!(row["last_sync_attempt"].as_i64().is_some_and(|value| value > 0));
    assert_eq!(row["sync_backoff"].as_u64(), Some(0));
    assert_eq!(row["next_sync_attempt"].as_i64(), Some(0));
    assert!(row["acceptance_rate"].as_f64().is_some_and(|value| value > 0.0));
}

#[test]
fn propagation_remote_sync_imports_payloads_into_local_store() {
    let payload = b"remote-sync-propagation-payload";
    let payload_hex = hex::encode(payload);
    let transient_id = hex::encode(Sha256::digest(payload));
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({
            "synced": true,
            "imported_count": 1,
            "messages": [{
                "transient_id": transient_id,
                "payload_hex": payload_hex,
            }],
        })),
    }));

    let result = daemon
        .handle_rpc(rpc_request(
            73,
            "propagation_remote_sync",
            json!({
                "remote": "remote-node",
                "peer": "peer-a",
            }),
        ))
        .expect("remote sync")
        .result
        .expect("remote sync result");
    assert_eq!(result["result"]["imported_count"].as_u64(), Some(1));
    assert_eq!(result["result"]["imported_ids"], json!([transient_id]));

    daemon.propagation_payloads.lock().expect("propagation payload mutex poisoned").clear();
    let fetched = daemon
        .handle_rpc(rpc_request(
            74,
            "propagation_fetch",
            json!({
                "transient_id": transient_id,
            }),
        ))
        .expect("local fetch after remote sync")
        .result
        .expect("local fetch result");
    assert_eq!(fetched["payload_hex"].as_str(), Some(payload_hex.as_str()));
}

#[test]
fn duplicate_propagation_remote_sync_import_does_not_double_count_received() {
    let payload = b"duplicate-remote-sync-propagation-payload";
    let payload_hex = hex::encode(payload);
    let transient_id = hex::encode(Sha256::digest(payload));
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({
            "synced": true,
            "imported_count": 1,
            "messages": [{
                "transient_id": transient_id,
                "payload_hex": payload_hex,
            }],
        })),
    }));

    let mut second = JsonValue::Null;
    for request_id in [73, 74] {
        let result = daemon
            .handle_rpc(rpc_request(
                request_id,
                "propagation_remote_sync",
                json!({
                    "remote": "remote-node",
                    "peer": "peer-a",
                }),
            ))
            .expect("remote sync")
            .result
            .expect("remote sync result");
        second = result;
    }
    assert_eq!(second["result"]["imported_count"].as_u64(), Some(0));
    assert_eq!(second["result"]["imported_ids"], json!([]));

    let status = daemon
        .handle_rpc(RpcRequest { id: 75, method: "propagation_status".to_string(), params: None })
        .expect("propagation status")
        .result
        .expect("propagation status result");
    assert_eq!(
        status["propagation"]["client_propagation_messages_received"].as_u64(),
        Some(1)
    );
    assert_eq!(status["propagation"]["total_ingested"].as_u64(), Some(1));
    assert_eq!(status["propagation"]["last_ingest_count"].as_u64(), Some(0));
}

#[test]
fn propagation_remote_fetch_imports_payloads_into_local_store() {
    let payload = b"remote-propagation-payload";
    let payload_hex = hex::encode(payload);
    let transient_id = hex::encode(Sha256::digest(payload));
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({
            "available_count": 1,
            "fetched_count": 1,
            "imported_count": 1,
            "messages": [{
                "transient_id": transient_id,
                "destination": "23".repeat(16),
                "payload_hex": payload_hex,
                "received_at": 1_700_000_700i64,
                "stamp_value": 6,
            }],
        })),
    }));

    let result = daemon
        .handle_rpc(rpc_request(
            73,
            "propagation_remote_fetch",
            json!({
                "remote": "remote-node",
            }),
        ))
        .expect("remote fetch")
        .result
        .expect("remote fetch result");
    assert_eq!(result["propagation"]["sync_state"].as_u64(), Some(0x07));
    assert_eq!(result["propagation"]["state_name"].as_str(), Some("completed"));
    assert_eq!(result["propagation"]["sync_progress"].as_f64(), Some(1.0));
    assert!(result["propagation"]["last_sync_started"].as_i64().is_some());
    assert!(result["propagation"]["last_sync_completed"].as_i64().is_some());
    assert_eq!(result["propagation"]["last_sync_error"], JsonValue::Null);
    assert_eq!(result["result"]["imported_count"].as_u64(), Some(1));
    assert_eq!(result["result"]["imported_ids"], json!([transient_id]));

    daemon.propagation_payloads.lock().expect("propagation payload mutex poisoned").clear();
    let fetched = daemon
        .handle_rpc(rpc_request(
            74,
            "propagation_fetch",
            json!({
                "transient_id": transient_id,
            }),
        ))
        .expect("local fetch after remote import")
        .result
        .expect("local fetch result");
    assert_eq!(fetched["payload_hex"].as_str(), Some(payload_hex.as_str()));
}

#[test]
fn propagation_remote_fetch_updates_lifecycle_status() {
    let payload = b"remote-fetch-lifecycle-payload";
    let payload_hex = hex::encode(payload);
    let transient_id = hex::encode(Sha256::digest(payload));
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({
            "available_count": 1,
            "fetched_count": 1,
            "imported_count": 1,
            "messages": [{
                "transient_id": transient_id,
                "payload_hex": payload_hex,
            }],
        })),
    }));

    daemon
        .handle_rpc(rpc_request(
            75,
            "propagation_remote_fetch",
            json!({
                "remote": "remote-node",
            }),
        ))
        .expect("remote fetch");

    let status = daemon
        .handle_rpc(RpcRequest { id: 76, method: "propagation_status".to_string(), params: None })
        .expect("propagation status")
        .result
        .expect("propagation status result");
    let propagation = &status["propagation"];
    assert_eq!(propagation["sync_state"].as_u64(), Some(0x07));
    assert_eq!(propagation["state_name"].as_str(), Some("completed"));
    assert_eq!(propagation["sync_progress"].as_f64(), Some(1.0));
    assert!(propagation["last_sync_started"].as_i64().is_some());
    assert!(propagation["last_sync_completed"].as_i64().is_some());
    assert_eq!(propagation["last_sync_error"], JsonValue::Null);
}

#[test]
fn propagation_remote_fetch_derives_missing_transient_id_from_payload_bytes() {
    let payload = b"remote-payload-without-id";
    let payload_hex = hex::encode(payload);
    let transient_id = hex::encode(Sha256::digest(payload));
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({
            "available_count": 1,
            "fetched_count": 1,
            "imported_count": 1,
            "payloads": [{
                "payload_hex": payload_hex,
            }],
        })),
    }));

    daemon
        .handle_rpc(rpc_request(
            74,
            "propagation_remote_fetch",
            json!({
                "remote": "remote-node",
            }),
        ))
        .expect("remote fetch");

    daemon.propagation_payloads.lock().expect("propagation payload mutex poisoned").clear();
    let fetched = daemon
        .handle_rpc(rpc_request(
            75,
            "propagation_fetch",
            json!({
                "transient_id": transient_id,
            }),
        ))
        .expect("local fetch after remote import")
        .result
        .expect("local fetch result");
    assert_eq!(fetched["payload_hex"].as_str(), Some(payload_hex.as_str()));
}

#[test]
fn propagation_remote_fetch_rejects_mismatched_transient_id() {
    let payload_hex = hex::encode(b"remote-payload-with-mismatched-id");
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({
            "available_count": 1,
            "fetched_count": 1,
            "imported_count": 1,
            "messages": [{
                "transient_id": "aa".repeat(32),
                "payload_hex": payload_hex,
            }],
        })),
    }));

    let err = daemon
        .handle_rpc(rpc_request(
            76,
            "propagation_remote_fetch",
            json!({
                "remote": "remote-node",
            }),
        ))
        .expect_err("mismatched remote transient_id must be rejected");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(err.to_string().contains("transient_id does not match propagation payload"));
    assert!(
        daemon
            .store
            .get_propagation_entry("aa".repeat(32).as_str())
            .expect("load bogus transient id")
            .is_none()
    );
}

#[test]
fn failed_propagation_remote_fetch_import_updates_lifecycle_error() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({
            "available_count": 1,
            "fetched_count": 1,
            "imported_count": 1,
            "messages": [{
                "payload_hex": "not-hex",
            }],
        })),
    }));

    let err = daemon
        .handle_rpc(rpc_request(
            77,
            "propagation_remote_fetch",
            json!({
                "remote": "remote-node",
            }),
        ))
        .expect_err("remote fetch import failure should be returned");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(err.to_string().contains("invalid remote propagation payload hex"));

    let status = daemon
        .handle_rpc(RpcRequest { id: 78, method: "propagation_status".to_string(), params: None })
        .expect("propagation status")
        .result
        .expect("propagation status result");
    let propagation = &status["propagation"];
    assert_eq!(propagation["sync_state"].as_u64(), Some(0xfe));
    assert_eq!(propagation["state_name"].as_str(), Some("failed"));
    assert_eq!(propagation["sync_progress"].as_f64(), Some(0.0));
    assert!(propagation["last_sync_started"].as_i64().is_some());
    assert!(propagation["last_sync_completed"].is_null());
    assert!(propagation["last_sync_error"]
        .as_str()
        .is_some_and(|value| value.contains("invalid remote propagation payload hex")));
}

#[test]
fn propagation_remote_download_imports_payloads_into_local_store() {
    let payload = b"remote-download-propagation-payload";
    let payload_hex = hex::encode(payload);
    let transient_id = hex::encode(Sha256::digest(payload));
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({
            "downloaded_count": 1,
            "imported_count": 1,
            "messages": [{
                "transient_id": transient_id,
                "payload_hex": payload_hex,
            }],
        })),
    }));

    let result = daemon
        .handle_rpc(rpc_request(
            76,
            "propagation_remote_download",
            json!({
                "remote": "remote-node",
            }),
        ))
        .expect("remote download")
        .result
        .expect("remote download result");
    assert_eq!(result["propagation"]["sync_state"].as_u64(), Some(0x07));
    assert_eq!(result["propagation"]["state_name"].as_str(), Some("completed"));
    assert_eq!(result["propagation"]["sync_progress"].as_f64(), Some(1.0));
    assert!(result["propagation"]["last_sync_started"].as_i64().is_some());
    assert!(result["propagation"]["last_sync_completed"].as_i64().is_some());
    assert_eq!(result["propagation"]["last_sync_error"], JsonValue::Null);
    assert_eq!(result["result"]["imported_count"].as_u64(), Some(1));
    assert_eq!(result["result"]["imported_ids"], json!([transient_id]));

    daemon.propagation_payloads.lock().expect("propagation payload mutex poisoned").clear();
    let fetched = daemon
        .handle_rpc(rpc_request(
            77,
            "propagation_fetch",
            json!({
                "transient_id": transient_id,
            }),
        ))
        .expect("local fetch after remote download")
        .result
        .expect("local fetch result");
    assert_eq!(fetched["payload_hex"].as_str(), Some(payload_hex.as_str()));
}

#[test]
fn propagation_remote_download_forwards_transfer_limit_to_bridge() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TransferLimitRemoteControlBridge));

    daemon
        .handle_rpc(rpc_request(
            77,
            "propagation_remote_download",
            json!({
                "remote": "remote-node",
                "transfer_limit_kb": 42.5,
            }),
        ))
        .expect("remote download with transfer limit");
}

#[test]
fn propagation_remote_sync_forwards_transfer_limit_to_bridge() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TransferLimitRemoteControlBridge));

    daemon
        .handle_rpc(rpc_request(
            78,
            "propagation_remote_sync",
            json!({
                "remote": "remote-node",
                "peer": "peer-transfer-limit",
                "transfer_limit_kb": 42.5,
            }),
        ))
        .expect("remote sync with transfer limit");
}

#[test]
fn failed_propagation_remote_download_import_updates_lifecycle_error() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({
            "downloaded_count": 1,
            "imported_count": 1,
            "messages": [{
                "payload_hex": "not-hex",
            }],
        })),
    }));

    let err = daemon
        .handle_rpc(rpc_request(
            78,
            "propagation_remote_download",
            json!({
                "remote": "remote-node",
            }),
        ))
        .expect_err("remote download import failure should be returned");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(err.to_string().contains("invalid remote propagation payload hex"));

    let status = daemon
        .handle_rpc(RpcRequest { id: 79, method: "propagation_status".to_string(), params: None })
        .expect("propagation status")
        .result
        .expect("propagation status result");
    let propagation = &status["propagation"];
    assert_eq!(propagation["sync_state"].as_u64(), Some(0xfe));
    assert_eq!(propagation["state_name"].as_str(), Some("failed"));
    assert_eq!(propagation["sync_progress"].as_f64(), Some(0.0));
    assert!(propagation["last_sync_started"].as_i64().is_some());
    assert!(propagation["last_sync_completed"].is_null());
    assert!(propagation["last_sync_error"]
        .as_str()
        .is_some_and(|value| value.contains("invalid remote propagation payload hex")));
}

#[test]
fn failed_propagation_remote_sync_updates_lifecycle_error() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Err(std::io::ErrorKind::TimedOut),
    }));

    let err = daemon
        .handle_rpc(rpc_request(
            74,
            "propagation_remote_sync",
            json!({
                "remote": "remote-node",
                "peer": "peer-a",
            }),
        ))
        .expect_err("remote sync failure should be returned");
    assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);

    let status = daemon
        .handle_rpc(RpcRequest { id: 75, method: "propagation_status".to_string(), params: None })
        .expect("propagation status")
        .result
        .expect("propagation status result");
    let propagation = &status["propagation"];
    assert_eq!(propagation["sync_state"].as_u64(), Some(0xfe));
    assert_eq!(propagation["state_name"].as_str(), Some("failed"));
    assert_eq!(propagation["sync_progress"].as_f64(), Some(0.0));
    assert!(propagation["last_sync_started"].as_i64().is_some());
    assert!(propagation["last_sync_completed"].is_null());
    assert_eq!(propagation["last_sync_error"].as_str(), Some("remote sync failed"));
}

#[test]
fn failed_propagation_remote_sync_updates_peer_backoff() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Err(std::io::ErrorKind::TimedOut),
    }));
    daemon
        .handle_rpc(rpc_request(75, "peer_sync", json!({ "peer": "peer-remote-sync-fail" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-remote-sync-fail").expect("peer record");
        peer.alive = true;
        peer.sync_backoff = 0;
        peer.next_sync_attempt = 0;
        peer.acceptance_rate = 0.5;
    }

    let err = daemon
        .handle_rpc(rpc_request(
            76,
            "propagation_remote_sync",
            json!({
                "remote": "remote-node",
                "peer": "peer-remote-sync-fail",
            }),
        ))
        .expect_err("remote sync failure should be returned");
    assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);

    let peers = daemon
        .handle_rpc(RpcRequest { id: 77, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-remote-sync-fail"))
        .expect("peer row");
    assert_eq!(row["alive"].as_bool(), Some(false));
    assert_eq!(row["sync_backoff"].as_u64(), Some(12 * 60));
    let last_sync_attempt = row["last_sync_attempt"].as_i64().expect("last sync attempt");
    assert!(last_sync_attempt > 0);
    assert_eq!(row["next_sync_attempt"].as_i64(), Some(last_sync_attempt + 12 * 60));
    assert!(row["acceptance_rate"].as_f64().is_some_and(|value| value < 0.5));
}

#[test]
fn failed_propagation_remote_sync_import_updates_peer_backoff() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({
            "synced": true,
            "messages": [{
                "payload_hex": "not-hex",
            }],
        })),
    }));
    daemon
        .handle_rpc(rpc_request(78, "peer_sync", json!({ "peer": "peer-remote-sync-import-fail" })))
        .expect("initial peer sync");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        let peer = peers.get_mut("peer-remote-sync-import-fail").expect("peer record");
        peer.alive = true;
        peer.sync_backoff = 0;
        peer.next_sync_attempt = 0;
        peer.acceptance_rate = 0.5;
    }

    let err = daemon
        .handle_rpc(rpc_request(
            79,
            "propagation_remote_sync",
            json!({
                "remote": "remote-node",
                "peer": "peer-remote-sync-import-fail",
            }),
        ))
        .expect_err("remote sync import failure should be returned");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(err.to_string().contains("invalid remote propagation payload hex"));

    let peers = daemon
        .handle_rpc(RpcRequest { id: 80, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-remote-sync-import-fail"))
        .expect("peer row");
    assert_eq!(row["alive"].as_bool(), Some(false));
    assert_eq!(row["sync_backoff"].as_u64(), Some(12 * 60));
    let last_sync_attempt = row["last_sync_attempt"].as_i64().expect("last sync attempt");
    assert!(last_sync_attempt > 0);
    assert_eq!(row["next_sync_attempt"].as_i64(), Some(last_sync_attempt + 12 * 60));
    assert!(row["acceptance_rate"].as_f64().is_some_and(|value| value < 0.5));
}

#[test]
fn propagation_remote_unpeer_clears_local_peer_and_queue_state() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({})),
    }));
    daemon
        .handle_rpc(rpc_request(76, "peer_sync", json!({ "peer": "peer-remote-unpeer" })))
        .expect("peer sync");

    let entry = PropagationEntryRecord {
        transient_id: "e1".repeat(32),
        destination: "19".repeat(16),
        payload_hex: "19".repeat(24),
        received_at: 1_700_000_801,
        size_bytes: 24,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store propagation entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-remote-unpeer", entry.transient_id.as_str())
        .expect("mark unhandled");

    let result = daemon
        .handle_rpc(rpc_request(
            77,
            "propagation_remote_unpeer",
            json!({
                "remote": "remote-node",
                "peer": "peer-remote-unpeer",
            }),
        ))
        .expect("remote unpeer")
        .result
        .expect("remote unpeer result");
    assert_eq!(result["removed"].as_bool(), Some(true));
    assert_eq!(result["propagation_cleared"].as_u64(), Some(1));
    assert_eq!(result["propagation_cleared_bytes"].as_u64(), Some(24));
    assert_eq!(result["messages"]["unhandled"].as_u64(), Some(1));
    assert_eq!(result["messages"]["unhandled_bytes"].as_u64(), Some(24));

    let peers = daemon
        .handle_rpc(RpcRequest { id: 78, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    assert_eq!(peers["peers"].as_array().map(Vec::len), Some(0));
    assert!(
        daemon
            .store
            .list_peer_unhandled_propagation("peer-remote-unpeer")
            .expect("list unhandled")
            .is_empty()
    );
}

#[test]
fn propagation_remote_unpeer_publishes_peer_removed_event() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({})),
    }));
    daemon
        .handle_rpc(rpc_request(79, "peer_sync", json!({ "peer": "peer-remote-unpeer-event" })))
        .expect("peer sync");
    daemon.event_queue.lock().expect("event_queue mutex poisoned").clear();

    daemon
        .handle_rpc(rpc_request(
            80,
            "propagation_remote_unpeer",
            json!({
                "remote": "remote-node",
                "peer": "peer-remote-unpeer-event",
            }),
        ))
        .expect("remote unpeer");

    let event = daemon
        .event_queue
        .lock()
        .expect("event_queue mutex poisoned")
        .iter()
        .rev()
        .find(|event| event.event_type == "peer_unpeer")
        .cloned()
        .expect("peer unpeer event");
    assert_eq!(event.payload["peer"].as_str(), Some("peer-remote-unpeer-event"));
    assert_eq!(event.payload["remote"].as_str(), Some("remote-node"));
    assert_eq!(event.payload["removed"].as_bool(), Some(true));
    assert_eq!(event.payload["propagation_cleared"].as_u64(), Some(0));
    assert_eq!(event.payload["messages"]["unhandled"].as_u64(), Some(0));
}

#[test]
fn failed_propagation_remote_unpeer_preserves_local_peer_and_queue_state() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Err(std::io::ErrorKind::TimedOut),
    }));
    daemon
        .handle_rpc(rpc_request(79, "peer_sync", json!({ "peer": "peer-remote-unpeer-fail" })))
        .expect("peer sync");

    let entry = PropagationEntryRecord {
        transient_id: "e2".repeat(32),
        destination: "1a".repeat(16),
        payload_hex: "1a".repeat(20),
        received_at: 1_700_000_802,
        size_bytes: 20,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store propagation entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-remote-unpeer-fail", entry.transient_id.as_str())
        .expect("mark unhandled");

    let err = daemon
        .handle_rpc(rpc_request(
            80,
            "propagation_remote_unpeer",
            json!({
                "remote": "remote-node",
                "peer": "peer-remote-unpeer-fail",
            }),
        ))
        .expect_err("remote unpeer failure should be returned");
    assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
    assert_eq!(err.to_string(), "remote unpeer failed");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 81, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"].as_array().and_then(|rows| rows.first()).expect("peer row");
    assert_eq!(row["peer"].as_str(), Some("peer-remote-unpeer-fail"));
    assert_eq!(row["messages"]["unhandled"].as_u64(), Some(1));
    assert_eq!(row["messages"]["unhandled_bytes"].as_u64(), Some(20));
    assert_eq!(
        daemon
            .store
            .list_peer_unhandled_propagation("peer-remote-unpeer-fail")
            .expect("list unhandled"),
        vec![entry]
    );
}

#[test]
fn failed_propagation_remote_sync_clears_previous_completion() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({"synced": true})),
    }));
    daemon
        .handle_rpc(rpc_request(
            76,
            "propagation_remote_sync",
            json!({
                "remote": "remote-node",
                "peer": "peer-a",
            }),
        ))
        .expect("initial remote sync");

    let completed = daemon
        .handle_rpc(RpcRequest { id: 77, method: "propagation_status".to_string(), params: None })
        .expect("propagation status")
        .result
        .expect("propagation status result");
    assert!(completed["propagation"]["last_sync_completed"].as_i64().is_some());

    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Err(std::io::ErrorKind::TimedOut),
    }));
    let err = daemon
        .handle_rpc(rpc_request(
            78,
            "propagation_remote_sync",
            json!({
                "remote": "remote-node",
                "peer": "peer-a",
            }),
        ))
        .expect_err("second remote sync should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);

    let failed = daemon
        .handle_rpc(RpcRequest { id: 79, method: "propagation_status".to_string(), params: None })
        .expect("propagation status")
        .result
        .expect("propagation status result");
    let propagation = &failed["propagation"];
    assert_eq!(propagation["sync_state"].as_u64(), Some(0xfe));
    assert_eq!(propagation["state_name"].as_str(), Some("failed"));
    assert!(propagation["last_sync_completed"].is_null());
    assert_eq!(propagation["last_sync_error"].as_str(), Some("remote sync failed"));
}

#[test]
fn propagation_acknowledge_sync_completion_resets_completed_state_like_python() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Ok(json!({"synced": true})),
    }));
    daemon
        .handle_rpc(rpc_request(
            80,
            "propagation_remote_sync",
            json!({
                "remote": "remote-node",
                "peer": "peer-a",
            }),
        ))
        .expect("remote sync");

    let acknowledged = daemon
        .handle_rpc(rpc_request(
            81,
            "propagation_acknowledge_sync_completion",
            json!({}),
        ))
        .expect("acknowledge sync")
        .result
        .expect("acknowledge result");
    let propagation = &acknowledged["propagation"];
    assert_eq!(propagation["sync_state"].as_u64(), Some(0x00));
    assert_eq!(propagation["state_name"].as_str(), Some("idle"));
    assert_eq!(propagation["sync_progress"].as_f64(), Some(0.0));
    assert!(propagation["last_sync_completed"].as_i64().is_some());
    assert_eq!(propagation["last_sync_error"], JsonValue::Null);
}

#[test]
fn propagation_acknowledge_sync_completion_preserves_failure_without_reset() {
    let daemon = RpcDaemon::test_instance();
    daemon.set_remote_control_bridge(Arc::new(TestRemoteControlBridge {
        result: Err(std::io::ErrorKind::TimedOut),
    }));
    daemon
        .handle_rpc(rpc_request(
            82,
            "propagation_remote_sync",
            json!({
                "remote": "remote-node",
                "peer": "peer-a",
            }),
        ))
        .expect_err("remote sync failure should be returned");

    let acknowledged = daemon
        .handle_rpc(rpc_request(
            83,
            "propagation_acknowledge_sync_completion",
            json!({}),
        ))
        .expect("acknowledge failed sync")
        .result
        .expect("acknowledge result");
    let propagation = &acknowledged["propagation"];
    assert_eq!(propagation["sync_state"].as_u64(), Some(0xfe));
    assert_eq!(propagation["state_name"].as_str(), Some("failed"));
    assert_eq!(propagation["sync_progress"].as_f64(), Some(0.0));

    let reset = daemon
        .handle_rpc(rpc_request(
            84,
            "propagation_acknowledge_sync_completion",
            json!({ "reset_state": true }),
        ))
        .expect("reset failed sync")
        .result
        .expect("reset result");
    let propagation = &reset["propagation"];
    assert_eq!(propagation["sync_state"].as_u64(), Some(0x00));
    assert_eq!(propagation["state_name"].as_str(), Some("idle"));
    assert_eq!(propagation["last_sync_error"], JsonValue::Null);
}

#[test]
fn peer_types_drive_python_style_peer_counts() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            70,
            "propagation_enable",
            json!({
                "enabled": true,
                "static_peers": ["peer-static"],
            }),
        ))
        .expect("enable propagation");

    daemon
        .handle_rpc(rpc_request(71, "peer_sync", json!({ "peer": "peer-static" })))
        .expect("sync static peer");
    daemon
        .handle_rpc(rpc_request(72, "peer_sync", json!({ "peer": "peer-manual" })))
        .expect("sync manual peer");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 73, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let rows = peers["peers"].as_array().expect("peer rows");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|row| row["peer_type"].as_str() == Some("static")));
    assert!(rows.iter().any(|row| row["peer_type"].as_str() == Some("manual")));
}

#[test]
fn list_peers_static_type_tracks_current_static_peer_config() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            74,
            "propagation_enable",
            json!({
                "enabled": true,
                "static_peers": ["peer-old"],
            }),
        ))
        .expect("enable old static peer");
    daemon
        .handle_rpc(rpc_request(75, "peer_sync", json!({ "peer": "peer-old" })))
        .expect("sync old static peer");
    daemon
        .handle_rpc(rpc_request(
            76,
            "propagation_enable",
            json!({
                "enabled": true,
                "static_peers": ["peer-new"],
            }),
        ))
        .expect("replace static peers");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 77, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let old = peers["peers"]
        .as_array()
        .expect("peer rows")
        .iter()
        .find(|row| row["peer"].as_str() == Some("peer-old"))
        .expect("old peer row");
    assert_eq!(old["peer_type"].as_str(), Some("static"));
    assert_eq!(old["type"].as_str(), Some("discovered"));
}

#[test]
fn unpeered_peers_do_not_consume_max_peer_capacity() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(
            80,
            "propagation_enable",
            json!({
                "enabled": true,
                "max_peers": 1,
            }),
        ))
        .expect("enable propagation");

    let first = daemon
        .handle_rpc(rpc_request(81, "peer_sync", json!({ "peer": "peer-a" })))
        .expect("sync peer-a");
    assert!(first.error.is_none());

    let blocked = daemon.handle_rpc(rpc_request(82, "peer_sync", json!({ "peer": "peer-b" })));
    assert!(blocked.is_err(), "second peer should be rejected while capacity is full");

    let unpeered = daemon
        .handle_rpc(rpc_request(83, "peer_unpeer", json!({ "peer": "peer-a" })))
        .expect("unpeer peer-a");
    assert!(unpeered.error.is_none());

    let replacement = daemon
        .handle_rpc(rpc_request(84, "peer_sync", json!({ "peer": "peer-b" })))
        .expect("sync replacement peer-b");
    assert!(replacement.error.is_none(), "replacement peer should be admitted after unpeer");

    let peers = daemon
        .handle_rpc(RpcRequest { id: 86, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let rows = peers["peers"].as_array().expect("peer rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["peer"].as_str(), Some("peer-b"));

    let status = daemon
        .handle_rpc(RpcRequest { id: 85, method: "daemon_status_ex".to_string(), params: None })
        .expect("daemon status")
        .result
        .expect("daemon status result");
    assert_eq!(status["peer_count"].as_u64(), Some(1));
}

#[test]
fn peer_unpeer_snapshot_count_ignores_unpeered_records() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(87, "peer_sync", json!({ "peer": "peer-active" })))
        .expect("sync active peer");
    {
        let mut guard = daemon.peers.lock().expect("peers mutex poisoned");
        guard.insert(
            "peer-unpeered".to_string(),
            daemon.transient_peer_record(
                "peer-unpeered".to_string(),
                1_700_000_900,
                Vec::new(),
                None,
                None,
                Some("unpeered".to_string()),
            ),
        );
    }

    daemon
        .handle_rpc(rpc_request(88, "peer_unpeer", json!({ "peer": "peer-active" })))
        .expect("unpeer active peer");

    let status = daemon
        .handle_rpc(RpcRequest { id: 89, method: "daemon_status_ex".to_string(), params: None })
        .expect("daemon status")
        .result
        .expect("daemon status result");
    assert_eq!(status["peer_count"].as_u64(), Some(0));
}

#[test]
fn peer_unpeer_clears_persisted_propagation_queue_marks() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(90, "peer_sync", json!({ "peer": "peer-unpeer-queue" })))
        .expect("sync peer");
    let entry = PropagationEntryRecord {
        transient_id: "ee".repeat(32),
        destination: "19".repeat(16),
        payload_hex: "19".repeat(24),
        received_at: 1_700_000_920,
        size_bytes: 24,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&entry).expect("store propagation entry");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-unpeer-queue", entry.transient_id.as_str())
        .expect("mark unhandled");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-unpeer-queue", "fa".repeat(32).as_str())
        .expect("mark stale unhandled");

    let unpeer = daemon
        .handle_rpc(rpc_request(91, "peer_unpeer", json!({ "peer": "peer-unpeer-queue" })))
        .expect("unpeer peer")
        .result
        .expect("unpeer result");
    assert_eq!(unpeer["removed"].as_bool(), Some(true));
    assert_eq!(unpeer["propagation_cleared"].as_u64(), Some(1));
    assert_eq!(unpeer["propagation_cleared_bytes"].as_u64(), Some(24));
    assert_eq!(unpeer["messages"]["offered"].as_u64(), Some(1));
    assert_eq!(unpeer["messages"]["unhandled"].as_u64(), Some(1));
    assert!(
        daemon
            .store
            .list_peer_unhandled_propagation("peer-unpeer-queue")
            .expect("list unhandled")
            .is_empty()
    );

    daemon
        .handle_rpc(rpc_request(92, "peer_sync", json!({ "peer": "peer-unpeer-queue" })))
        .expect("resync peer");
    let peers = daemon
        .handle_rpc(RpcRequest { id: 93, method: "list_peers".to_string(), params: None })
        .expect("list peers")
        .result
        .expect("list peers result");
    let row = peers["peers"].as_array().and_then(|rows| rows.first()).expect("peer row");
    assert_eq!(row["messages"]["offered"].as_u64(), Some(0));
    assert_eq!(row["messages"]["unhandled"].as_u64(), Some(0));
}

#[test]
fn peer_unpeer_reports_cleared_propagation_queue_accounting() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .handle_rpc(rpc_request(93, "peer_sync", json!({ "peer": "peer-unpeer-accounting" })))
        .expect("sync peer");
    daemon.event_queue.lock().expect("event_queue mutex poisoned").clear();

    let handled = PropagationEntryRecord {
        transient_id: "c8".repeat(32),
        destination: "15".repeat(16),
        payload_hex: "15".repeat(12),
        received_at: 1_700_000_701,
        size_bytes: 12,
        stamp_value: None,
    };
    let unhandled = PropagationEntryRecord {
        transient_id: "c9".repeat(32),
        destination: "16".repeat(16),
        payload_hex: "16".repeat(24),
        received_at: 1_700_000_702,
        size_bytes: 24,
        stamp_value: None,
    };
    daemon.store.upsert_propagation_entry(&handled).expect("store handled entry");
    daemon.store.upsert_propagation_entry(&unhandled).expect("store unhandled entry");
    daemon
        .store
        .mark_peer_handled_propagation("peer-unpeer-accounting", handled.transient_id.as_str())
        .expect("mark handled");
    daemon
        .store
        .mark_peer_unhandled_propagation("peer-unpeer-accounting", unhandled.transient_id.as_str())
        .expect("mark unhandled");

    let result = daemon
        .handle_rpc(rpc_request(
            94,
            "peer_unpeer",
            json!({ "peer": "peer-unpeer-accounting" }),
        ))
        .expect("unpeer")
        .result
        .expect("unpeer result");
    assert_eq!(result["peer"].as_str(), Some("peer-unpeer-accounting"));
    assert_eq!(result["propagation_cleared"].as_u64(), Some(2));
    assert_eq!(result["propagation_cleared_bytes"].as_u64(), Some(36));
    assert_eq!(result["messages"]["offered"].as_u64(), Some(2));
    assert_eq!(result["messages"]["unhandled"].as_u64(), Some(1));
    assert_eq!(result["messages"]["offered_bytes"].as_u64(), Some(36));
    assert_eq!(result["messages"]["unhandled_bytes"].as_u64(), Some(24));
    assert_eq!(
        result["messages"]["handled_ids"].as_array().expect("result handled ids"),
        &[json!(handled.transient_id.as_str())]
    );
    assert_eq!(
        result["messages"]["unhandled_ids"].as_array().expect("result unhandled ids"),
        &[json!(unhandled.transient_id.as_str())]
    );

    let event = daemon
        .event_queue
        .lock()
        .expect("event_queue mutex poisoned")
        .iter()
        .rev()
        .find(|event| event.event_type == "peer_unpeer")
        .cloned()
        .expect("peer unpeer event");
    assert_eq!(event.payload["propagation_cleared"].as_u64(), Some(2));
    assert_eq!(event.payload["propagation_cleared_bytes"].as_u64(), Some(36));
    assert_eq!(event.payload["messages"]["offered"].as_u64(), Some(2));
    assert_eq!(event.payload["messages"]["unhandled"].as_u64(), Some(1));
    assert_eq!(event.payload["messages"]["offered_bytes"].as_u64(), Some(36));
    assert_eq!(event.payload["messages"]["unhandled_bytes"].as_u64(), Some(24));
    assert_eq!(
        event.payload["messages"]["handled_ids"].as_array().expect("event handled ids"),
        &[json!(handled.transient_id.as_str())]
    );
    assert_eq!(
        event.payload["messages"]["unhandled_ids"].as_array().expect("event unhandled ids"),
        &[json!(unhandled.transient_id.as_str())]
    );
}

#[test]
fn peer_sync_rejects_blank_peer_identifier() {
    let daemon = RpcDaemon::test_instance();

    let err = daemon
        .handle_rpc(rpc_request(94, "peer_sync", json!({ "peer": "   " })))
        .expect_err("blank peer id should be rejected");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(err.to_string().contains("peer is required"));
}

#[test]
fn lxmf_metadata_entries_merge_without_changing_receipt_status() {
    let daemon = RpcDaemon::test_instance();
    daemon
        .accept_inbound(MessageRecord {
            id: "metadata-message".to_string(),
            source: "source".to_string(),
            destination: "destination".to_string(),
            title: "title".to_string(),
            content: "content".to_string(),
            timestamp: 1_700_000_000,
            direction: "out".to_string(),
            fields: Some(json!({
                "app": "value",
                "_lxmf": {
                    "existing": true,
                },
            })),
            receipt_status: Some("sending".to_string()),
        })
        .expect("insert message");

    daemon
        .record_message_lxmf_metadata_entries(
            "metadata-message",
            [
                ("propagation_packed".to_string(), json!(true)),
                ("propagation_packed_size".to_string(), json!(1234)),
                ("propagation_stamp_value".to_string(), json!(19)),
            ],
        )
        .expect("record metadata");

    let result = daemon
        .handle_rpc(RpcRequest { id: 91, method: "list_messages".to_string(), params: None })
        .expect("list messages")
        .result
        .expect("list messages result");
    let message = result["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .find(|message| message["id"].as_str() == Some("metadata-message"))
        .expect("metadata message");

    assert_eq!(message["receipt_status"].as_str(), Some("sending"));
    assert_eq!(message["fields"]["app"].as_str(), Some("value"));
    assert_eq!(message["fields"]["_lxmf"]["existing"].as_bool(), Some(true));
    assert_eq!(message["fields"]["_lxmf"]["propagation_packed"].as_bool(), Some(true));
    assert_eq!(message["fields"]["_lxmf"]["propagation_packed_size"].as_u64(), Some(1234));
    assert_eq!(message["fields"]["_lxmf"]["propagation_stamp_value"].as_u64(), Some(19));
}
