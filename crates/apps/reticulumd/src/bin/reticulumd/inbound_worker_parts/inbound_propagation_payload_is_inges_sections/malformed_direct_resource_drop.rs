    #[tokio::test]
    async fn malformed_direct_delivery_resource_emits_drop_event_without_side_effects() {
        let daemon = RpcDaemon::test_instance();
        let delivery_private = PrivateIdentity::new_from_rand(OsRng);
        let delivery_destination = SingleInputDestination::new(
            delivery_private.clone(),
            DestinationName::new("lxmf", "delivery"),
        );
        let mut destination_hash = [0u8; 16];
        destination_hash.copy_from_slice(delivery_destination.desc.address_hash.as_slice());
        let delivery_core_private = to_core_private_identity(&delivery_private);
        let transport_identity = to_transport_private_identity(&delivery_core_private);
        let transport = Transport::new(TransportConfig::new("test", &transport_identity, true));
        let malformed_payload = b"not-a-valid-lxmf-resource-payload";

        delivery_events::accept_delivery_resource(
            &daemon,
            &transport,
            destination_hash,
            malformed_payload,
        )
        .await;

        let event = daemon.take_event().expect("drop event");
        assert_eq!(event.event_type, "inbound_dropped");
        assert_eq!(event.payload["reason"], json!("decode_failed"));
        assert_eq!(event.payload["delivery_kind"], json!("resource"));
        let raw_destination = hex::encode(destination_hash);
        assert!(event.payload["raw_destination_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:") && value != raw_destination));
        assert!(event.payload["resolved_destination_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:") && value != raw_destination));
        assert_eq!(event.payload["payload_mode"], json!("full_wire"));
        assert_eq!(event.payload["bytes_len"], json!(malformed_payload.len()));
        assert!(
            event.payload["detail"].as_str().is_some_and(|detail| detail.contains("full_wire")),
            "drop event should include bounded decode diagnostics: {:?}",
            event.payload
        );
        assert!(daemon.take_event().is_none(), "malformed resource should emit one event");

        let messages = daemon
            .handle_rpc(RpcRequest { id: 50, method: "list_messages".to_string(), params: None })
            .expect("list messages")
            .result
            .expect("list messages result");
        assert_eq!(messages["messages"].as_array().expect("message items").len(), 0);

        let peers = daemon
            .handle_rpc(RpcRequest { id: 51, method: "list_peers".to_string(), params: None })
            .expect("list peers")
            .result
            .expect("list peers result");
        assert_eq!(peers["peers"].as_array().expect("peer rows").len(), 0);
    }
