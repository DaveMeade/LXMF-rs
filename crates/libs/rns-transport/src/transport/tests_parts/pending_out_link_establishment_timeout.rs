/// `RNS.Link`'s watchdog closes a link that outlives its establishment timeout
/// while still pending, and on a non-transport instance the path it was
/// tried on is expired and requested again — the same handling a link that
/// was closed by hand already got here. Without it the request was repeated
/// every `INTERVAL_OUTPUT_LINK_REPEAT` for the life of the process.
#[tokio::test]
async fn a_pending_out_link_that_outlives_its_establishment_timeout_is_closed_and_the_path_expired() {
    let local_identity = PrivateIdentity::new_from_rand(OsRng);
    let config = TransportConfig::new("test", &local_identity, true);
    let transport = Transport::new(config);
    let handler = transport.get_handler();
    let mut iface_channel = transport.iface_manager().lock().await.new_channel(16);
    let iface = *iface_channel.address();
    let remote_identity = PrivateIdentity::new_from_rand(OsRng);
    let destination =
        SingleInputDestination::new(remote_identity, DestinationName::new("lxmf", "delivery"));
    let destination_hash = destination.desc.address_hash;

    {
        let mut guard = handler.lock().await;
        assert!(guard.path_table.restore_tunnel_path(
            destination_hash,
            destination_hash,
            1,
            iface,
            Hash::new_from_slice(b"packet"),
            std::time::Instant::now(),
        ));
    }

    let link = transport.link(destination.desc).await;
    let _initial_request = timeout(Duration::from_millis(200), iface_channel.tx_channel.recv())
        .await
        .expect("initial link request should be queued")
        .expect("tx channel open");
    // Well past the first hop plus one hop it was sized for.
    link.lock().await.set_created_at_for_test(std::time::Instant::now() - Duration::from_secs(60));

    super::jobs::handle_check_links(handler.lock().await).await;

    assert_eq!(link.lock().await.status(), LinkStatus::Closed, "the link is closed, not requested again");
    assert!(!handler.lock().await.out_links.contains_key(&destination_hash));
    assert!(handler.lock().await.path_table.get(&destination_hash).is_none(), "the path it was tried on is expired");
    let rediscovery = timeout(Duration::from_millis(200), iface_channel.tx_channel.recv())
        .await
        .expect("rediscovery path request should be queued")
        .expect("tx channel open");
    assert_eq!(rediscovery.tx_type, crate::iface::TxMessageType::Broadcast(None));
    assert_eq!(&rediscovery.packet.data.as_slice()[..ADDRESS_HASH_SIZE], destination_hash.as_slice());
}

/// Inside the timeout the link is left pending and its request repeated,
/// which is what the job did before.
#[tokio::test]
async fn a_pending_out_link_inside_its_establishment_timeout_is_requested_again() {
    let local_identity = PrivateIdentity::new_from_rand(OsRng);
    let config = TransportConfig::new("test", &local_identity, true);
    let transport = Transport::new(config);
    let handler = transport.get_handler();
    let mut iface_channel = transport.iface_manager().lock().await.new_channel(16);
    let iface = *iface_channel.address();
    let remote_identity = PrivateIdentity::new_from_rand(OsRng);
    let destination =
        SingleInputDestination::new(remote_identity, DestinationName::new("lxmf", "delivery"));
    let destination_hash = destination.desc.address_hash;

    {
        let mut guard = handler.lock().await;
        assert!(guard.path_table.restore_tunnel_path(
            destination_hash,
            destination_hash,
            1,
            iface,
            Hash::new_from_slice(b"packet"),
            std::time::Instant::now(),
        ));
    }

    let link = transport.link(destination.desc).await;
    let _initial_request = timeout(Duration::from_millis(200), iface_channel.tx_channel.recv())
        .await
        .expect("initial link request should be queued")
        .expect("tx channel open");
    // Past the repeat interval, inside the establishment timeout.
    link.lock().await.set_request_time_for_test(std::time::Instant::now() - Duration::from_secs(7));

    super::jobs::handle_check_links(handler.lock().await).await;

    assert_eq!(link.lock().await.status(), LinkStatus::Pending);
    assert!(handler.lock().await.path_table.get(&destination_hash).is_some(), "the path is kept");
    let repeat = timeout(Duration::from_millis(200), iface_channel.tx_channel.recv())
        .await
        .expect("the link request should be repeated")
        .expect("tx channel open");
    assert_eq!(repeat.packet.destination, destination_hash, "a repeated link request, not a path request");
}
