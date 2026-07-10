#[test]
fn sdk_identity_presence_list_filters_stale_peers_by_last_seen() {
    let daemon = RpcDaemon::test_instance();
    let fresh: PeerRecord = serde_json::from_value(json!({
        "destination_hash": "fresh-peer",
        "last_seen": 1_700_000_800,
        "last_heard": 1_700_000_800,
        "first_seen": 1_700_000_700,
        "seen_count": 2,
        "name": "Fresh Peer",
        "name_source": "announce",
        "alive": true,
    }))
    .expect("fresh peer record");
    let stale: PeerRecord = serde_json::from_value(json!({
        "destination_hash": "stale-peer",
        "last_seen": 1_700_000_100,
        "last_heard": 1_700_000_100,
        "first_seen": 1_700_000_000,
        "seen_count": 1,
        "name": "Stale Peer",
        "name_source": "announce",
        "alive": false,
    }))
    .expect("stale peer record");
    {
        let mut peers = daemon.peers.lock().expect("peers mutex poisoned");
        peers.insert(fresh.peer.clone(), fresh);
        peers.insert(stale.peer.clone(), stale);
    }

    let response = daemon
        .handle_rpc(rpc_request(
            12241,
            "sdk_identity_presence_list_v2",
            json!({ "cursor": null, "limit": 10, "min_last_seen_ts_ms": 1_700_000_500 }),
        ))
        .expect("filtered presence list");
    assert!(response.error.is_none());
    let result = response.result.expect("presence result");
    let rows = result["presence_list"]["peers"].as_array().expect("presence rows");
    assert!(rows.iter().any(|row| row["peer_id"] == json!("fresh-peer")));
    assert!(!rows.iter().any(|row| row["peer_id"] == json!("stale-peer")));
    assert!(rows
        .iter()
        .all(|row| row["last_seen_ts_ms"].as_i64().is_some_and(|last_seen| {
            last_seen >= 1_700_000_500
        })));
}

#[test]
fn sdk_peer_lifecycle_methods_roundtrip_through_daemon_dispatch() {
    let daemon = RpcDaemon::test_instance();
    let request = json!({
        "identity": "peer-lifecycle",
        "display_name": "RCH Relay",
        "correlation_id": "peer-life-corr",
        "metadata": {
            "callsign": "RCH-1",
            "capability_flags": ["rem.direct_chat"],
            "announce_slots": ["rch.broadcast"]
        },
        "extensions": {
            "source": "rem-rch"
        }
    });

    let connected = daemon
        .handle_rpc(rpc_request(12242, "sdk_peer_connect_v2", request.clone()))
        .expect("peer connect");
    assert!(connected.error.is_none());
    let connected_peer = connected.result.expect("connect result")["peer"].clone();
    assert_eq!(connected_peer["identity"], json!("peer-lifecycle"));
    assert_eq!(connected_peer["state"], json!("connected"));
    assert_eq!(connected_peer["connected"], json!(true));
    assert_eq!(connected_peer["display_name"], json!("RCH Relay"));
    assert_eq!(connected_peer["metadata"]["callsign"], json!("RCH-1"));
    assert_eq!(connected_peer["metadata"]["capability_flags"][0], json!("rem.direct_chat"));
    assert_eq!(connected_peer["metadata"]["announce_slots"][0], json!("rch.broadcast"));
    assert_eq!(connected_peer["extensions"]["source"], json!("rem-rch"));

    let disconnected = daemon
        .handle_rpc(rpc_request(12243, "sdk_peer_disconnect_v2", request.clone()))
        .expect("peer disconnect");
    assert!(disconnected.error.is_none());
    let disconnected_peer = disconnected.result.expect("disconnect result")["peer"].clone();
    assert_eq!(disconnected_peer["identity"], json!("peer-lifecycle"));
    assert_eq!(disconnected_peer["state"], json!("disconnected"));
    assert_eq!(disconnected_peer["connected"], json!(false));

    let reconnected = daemon
        .handle_rpc(rpc_request(12244, "sdk_peer_reconnect_v2", request))
        .expect("peer reconnect");
    assert!(reconnected.error.is_none());
    let reconnected_peer = reconnected.result.expect("reconnect result")["peer"].clone();
    assert_eq!(reconnected_peer["identity"], json!("peer-lifecycle"));
    assert_eq!(reconnected_peer["state"], json!("reconnected"));
    assert_eq!(reconnected_peer["connected"], json!(true));
}
