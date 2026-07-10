#[tokio::test]
async fn peer_local_propagation_destination_mismatch_emits_drop_and_still_relays() {
    let daemon = RpcDaemon::test_instance();
    let delivery_private = PrivateIdentity::new_from_rand(OsRng);
    let delivery_destination = Arc::new(TokioMutex::new(SingleInputDestination::new(
        delivery_private,
        DestinationName::new("lxmf", "delivery"),
    )));
    let mut destination_hash = [0u8; 16];
    {
        let destination = delivery_destination.lock().await;
        destination_hash.copy_from_slice(destination.desc.address_hash.as_slice());
    }
    daemon.set_delivery_destination_hash(Some(hex::encode(destination_hash)));
    let source_peer = hex::encode([0x84_u8; 16]);
    let relay_peer = hex::encode([0x85_u8; 16]);
    for (id, peer) in [(52, &source_peer), (53, &relay_peer)] {
        daemon
            .handle_rpc(RpcRequest {
                id,
                method: "peer_sync".to_string(),
                params: Some(json!({ "peer": peer })),
            })
            .expect("seed propagation peer");
    }
    while daemon.take_event().is_some() {}
    let mut transient_payload = vec![0xC5_u8; 16 + 33];
    transient_payload[..16].copy_from_slice(&[0x99_u8; 16]);
    let transient_id = hex::encode(Sha256::digest(&transient_payload));
    let envelope = rmp_serde::to_vec(&(1.0_f64, vec![transient_payload.clone()]))
        .expect("propagation envelope");

    let ingested = ingest_propagation_envelope_from_peer(
        &daemon,
        &envelope,
        Some(&delivery_destination),
        Some(&source_peer),
    )
    .await
    .expect("ingest peer mismatched propagation envelope");
    assert_eq!(ingested, 1);

    let event = daemon.take_event().expect("peer destination mismatch drop event");
    assert_eq!(event.event_type, "inbound_dropped");
    assert_eq!(event.payload["reason"], json!("destination_mismatch"));
    assert_eq!(event.payload["delivery_kind"], json!("propagation"));
    assert_eq!(event.payload["payload_mode"], json!("full_wire"));
    assert_eq!(event.payload["bytes_len"], json!(transient_payload.len()));
    assert_eq!(
        event.payload["detail"],
        json!("propagated LXMF payload is not addressed to the local delivery destination")
    );
    assert!(event.payload["raw_destination_hash"]
        .as_str()
        .is_some_and(|value| value.starts_with("sha256:") && value != hex::encode([0x99_u8; 16])));
    assert!(event.payload["resolved_destination_hash"]
        .as_str()
        .is_some_and(|value| value.starts_with("sha256:") && value != hex::encode(destination_hash)));
    let raw_hash = event.payload["raw_destination_hash"].as_str().expect("raw hash");
    let resolved_hash =
        event.payload["resolved_destination_hash"].as_str().expect("resolved hash");
    assert_ne!(raw_hash, resolved_hash);
    assert!(daemon.take_event().is_none(), "peer mismatch drop should emit one event");
    assert!(
        daemon
            .has_peer_completed_propagation_mark(source_peer.as_str(), transient_id.as_str())
            .expect("completed propagation mark lookup"),
        "nonlocal mismatched peer payloads should still mark the source peer handled"
    );
    let source = peer_row(&daemon, source_peer.as_str(), 54);
    assert_eq!(source["messages"]["incoming"].as_u64(), Some(1));
    assert_eq!(source["messages"]["unhandled_ids"], json!([]));
    let relay = peer_row(&daemon, relay_peer.as_str(), 55);
    assert_eq!(
        relay["messages"]["unhandled_ids"].as_array().expect("relay unhandled ids"),
        &[json!(transient_id.as_str())],
        "nonlocal mismatched peer payloads should still fan out to relay peers"
    );
    let messages = daemon
        .handle_rpc(RpcRequest { id: 56, method: "list_messages".to_string(), params: None })
        .expect("list messages")
        .result
        .expect("list messages result");
    assert!(messages["messages"].as_array().expect("message items").is_empty());
}
