    #[tokio::test]
    async fn duplicate_direct_delivery_packet_does_not_update_peer_activity_like_python() {
        let daemon = RpcDaemon::test_instance();
        let delivery_private = PrivateIdentity::new_from_rand(OsRng);
        let source_private = PrivateIdentity::new_from_rand(OsRng);
        let delivery_destination = SingleInputDestination::new(
            delivery_private.clone(),
            DestinationName::new("lxmf", "delivery"),
        );
        let source_destination = SingleInputDestination::new(
            source_private.clone(),
            DestinationName::new("lxmf", "delivery"),
        );
        let mut destination_hash = [0u8; 16];
        destination_hash.copy_from_slice(delivery_destination.desc.address_hash.as_slice());
        let mut source_hash = [0u8; 16];
        source_hash.copy_from_slice(source_destination.desc.address_hash.as_slice());
        let source_hex = hex::encode(source_hash);
        daemon.accept_announce(source_hex.clone(), 1).expect("accept source announce");
        let delivery_core_private = to_core_private_identity(&delivery_private);
        let transport_identity = to_transport_private_identity(&delivery_core_private);
        let transport = Transport::new(TransportConfig::new("test", &transport_identity, true));

        let wire = build_wire_message_with_options(
            source_hash,
            destination_hash,
            "duplicate direct title",
            "duplicate direct content",
            None,
            &to_core_private_identity(&source_private),
            None,
            None,
            None,
        )
        .expect("wire");

        delivery_events::accept_delivery_packet(
            &daemon,
            &transport,
            hex::encode(destination_hash).as_str(),
            destination_hash,
            &wire,
            ReceivedPayloadMode::FullWire,
        )
        .await;
        let after_first = peer_row(&daemon, source_hex.as_str(), 45);
        assert_eq!(after_first["rx_bytes"].as_u64(), Some(wire.len() as u64));
        assert_eq!(after_first["messages"]["incoming"].as_u64(), Some(1));
        while daemon.take_event().is_some() {}

        delivery_events::accept_delivery_packet(
            &daemon,
            &transport,
            hex::encode(destination_hash).as_str(),
            destination_hash,
            &wire,
            ReceivedPayloadMode::FullWire,
        )
        .await;

        let event = daemon.take_event().expect("duplicate direct drop event");
        assert_eq!(event.event_type, "inbound_dropped");
        assert_eq!(event.payload["reason"], json!("duplicate"));
        assert_eq!(event.payload["delivery_kind"], json!("packet"));
        let raw_destination = hex::encode(destination_hash);
        assert!(event.payload["raw_destination_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:") && value != raw_destination));
        assert!(event.payload["resolved_destination_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:") && value != raw_destination));
        assert_eq!(event.payload["payload_mode"], json!("full_wire"));
        assert_eq!(event.payload["bytes_len"], json!(wire.len()));
        assert!(event.payload["source_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:") && value != source_hex));
        assert!(event.payload["destination_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:") && value != raw_destination));
        assert!(event.payload["dropped_message_id"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")));
        assert_eq!(event.payload["detail"], json!("message already stored"));
        assert!(daemon.take_event().is_none(), "duplicate direct delivery should emit one event");

        let messages = daemon
            .handle_rpc(RpcRequest { id: 46, method: "list_messages".to_string(), params: None })
            .expect("list messages")
            .result
            .expect("list messages result");
        let items = messages["messages"].as_array().expect("message items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["fields"]["_lxmf"]["method"], json!(2));
        assert_eq!(items[0]["fields"]["_lxmf"]["transport_encrypted"], json!(true));
        assert_eq!(items[0]["fields"]["_lxmf"]["transport_encryption"], json!("Curve25519"));
        let after_second = peer_row(&daemon, source_hex.as_str(), 47);
        assert_eq!(after_second["rx_bytes"].as_u64(), Some(wire.len() as u64));
        assert_eq!(after_second["messages"]["incoming"].as_u64(), Some(1));
    }
