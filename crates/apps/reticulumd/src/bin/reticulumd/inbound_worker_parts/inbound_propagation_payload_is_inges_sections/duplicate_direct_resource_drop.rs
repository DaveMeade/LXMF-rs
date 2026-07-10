    #[tokio::test]
    async fn duplicate_direct_delivery_resource_emits_drop_event_without_duplicate_storage() {
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
        while daemon.take_event().is_some() {}
        let delivery_core_private = to_core_private_identity(&delivery_private);
        let transport_identity = to_transport_private_identity(&delivery_core_private);
        let transport = Transport::new(TransportConfig::new("test", &transport_identity, true));

        let wire = build_wire_message_with_options(
            source_hash,
            destination_hash,
            "duplicate resource title",
            "duplicate resource content",
            None,
            &to_core_private_identity(&source_private),
            None,
            None,
            None,
        )
        .expect("wire");

        delivery_events::accept_delivery_resource(&daemon, &transport, destination_hash, &wire)
            .await;
        let first_event = daemon.take_event().expect("first resource inbound event");
        assert_eq!(first_event.event_type, "inbound");
        assert!(daemon.take_event().is_none(), "first resource should emit one event");

        delivery_events::accept_delivery_resource(&daemon, &transport, destination_hash, &wire)
            .await;

        let messages = daemon
            .handle_rpc(RpcRequest { id: 48, method: "list_messages".to_string(), params: None })
            .expect("list messages")
            .result
            .expect("list messages result");
        let items = messages["messages"].as_array().expect("message items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["fields"]["_lxmf"]["method"], json!(2));
        assert_eq!(items[0]["fields"]["_lxmf"]["transport_encrypted"], json!(true));
        assert_eq!(items[0]["fields"]["_lxmf"]["transport_encryption"], json!("Curve25519"));

        let event = daemon.take_event().expect("duplicate resource drop event");
        assert_eq!(event.event_type, "inbound_dropped");
        assert_eq!(event.payload["reason"], json!("duplicate"));
        assert_eq!(event.payload["delivery_kind"], json!("resource"));
        let destination_hex = hex::encode(destination_hash);
        assert!(event.payload["raw_destination_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:") && value != destination_hex));
        assert!(event.payload["resolved_destination_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:") && value != destination_hex));
        assert_eq!(event.payload["payload_mode"], json!("full_wire"));
        assert_eq!(event.payload["bytes_len"], json!(wire.len()));
        assert!(event.payload["dropped_message_id"].as_str().is_some_and(|value| {
            value.starts_with("sha256:") && Some(value) != items[0]["id"].as_str()
        }));
        assert!(event.payload["source_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:") && value != source_hex));
        assert!(event.payload["destination_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:") && value != destination_hex));
        assert!(daemon.take_event().is_none(), "duplicate resource should emit one drop event");
    }
