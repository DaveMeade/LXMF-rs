/// A remote announce with no route on file is blocked on every mode, not just
/// the two whose `matches!(Some(..))` happens to reject `None`.
#[tokio::test]
async fn full_blocks_remote_announce_without_a_next_hop_interface() {
    let mut mgr = InterfaceManager::new(16);
    let mut rx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Unicast, InterfaceMode::Full)
        .tx_channel;
    let packet = announce_packet();
    let trace = mgr
        .send_with_announce_policy(
            TxMessage { tx_type: TxMessageType::Broadcast(None), packet: packet.clone() },
            Some(AnnounceBroadcastPolicy {
                local_destination: false,
                next_hop_iface_mode: None,
                next_hop_announces_to_internal: None,
            }),
        )
        .await;
    assert_eq!(trace.sent_ifaces, 0);
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn gateway_blocks_remote_announce_without_a_next_hop_interface() {
    let mut mgr = InterfaceManager::new(16);
    let mut rx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Unicast, InterfaceMode::Gateway)
        .tx_channel;
    let packet = announce_packet();
    let trace = mgr
        .send_with_announce_policy(
            TxMessage { tx_type: TxMessageType::Broadcast(None), packet: packet.clone() },
            Some(AnnounceBroadcastPolicy {
                local_destination: false,
                next_hop_iface_mode: None,
                next_hop_announces_to_internal: None,
            }),
        )
        .await;
    assert_eq!(trace.sent_ifaces, 0);
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn internal_blocks_remote_announce_without_a_next_hop_interface() {
    let mut mgr = InterfaceManager::new(16);
    let mut rx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Unicast, InterfaceMode::Internal)
        .tx_channel;
    let packet = announce_packet();
    let trace = mgr
        .send_with_announce_policy(
            TxMessage { tx_type: TxMessageType::Broadcast(None), packet: packet.clone() },
            Some(AnnounceBroadcastPolicy {
                local_destination: false,
                next_hop_iface_mode: None,
                next_hop_announces_to_internal: None,
            }),
        )
        .await;
    assert_eq!(trace.sent_ifaces, 0);
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn point_to_point_blocks_remote_announce_without_a_next_hop_interface() {
    let mut mgr = InterfaceManager::new(16);
    let mut rx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Unicast, InterfaceMode::PointToPoint)
        .tx_channel;
    let packet = announce_packet();
    let trace = mgr
        .send_with_announce_policy(
            TxMessage { tx_type: TxMessageType::Broadcast(None), packet: packet.clone() },
            Some(AnnounceBroadcastPolicy {
                local_destination: false,
                next_hop_iface_mode: None,
                next_hop_announces_to_internal: None,
            }),
        )
        .await;
    assert_eq!(trace.sent_ifaces, 0);
    assert!(rx.try_recv().is_err());
}

/// The guard is scoped to remote announces: this node's own destination has no
/// next hop by definition, and must still be announceable.
#[tokio::test]
async fn full_allows_local_announce_without_a_next_hop_interface() {
    let mut mgr = InterfaceManager::new(16);
    let mut rx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Unicast, InterfaceMode::Full)
        .tx_channel;
    let packet = announce_packet();
    let trace = mgr
        .send_with_announce_policy(
            TxMessage { tx_type: TxMessageType::Broadcast(None), packet: packet.clone() },
            Some(AnnounceBroadcastPolicy {
                local_destination: true,
                next_hop_iface_mode: None,
                next_hop_announces_to_internal: None,
            }),
        )
        .await;
    assert_eq!(trace.sent_ifaces, 1);
    assert_eq!(rx.try_recv().expect("announce").packet, packet);
}

/// An interface opting out of `announces_from_internal` refuses an announce
/// learned over an internal-mode next hop.
#[tokio::test]
async fn announces_from_internal_false_blocks_a_remote_announce_from_an_internal_next_hop() {
    let mut mgr = InterfaceManager::new(16);
    let mut rx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Unicast, InterfaceMode::Full)
        .tx_channel;
    let iface = mgr.ifaces[0].address;
    mgr.set_shared_config(
        iface,
        InterfaceSharedConfig { announces_from_internal: Some(false), ..Default::default() },
    );
    let packet = announce_packet();
    let trace = mgr
        .send_with_announce_policy(
            TxMessage { tx_type: TxMessageType::Broadcast(None), packet: packet.clone() },
            Some(AnnounceBroadcastPolicy {
                local_destination: false,
                next_hop_iface_mode: Some(InterfaceMode::Internal),
                next_hop_announces_to_internal: None,
            }),
        )
        .await;
    assert_eq!(trace.sent_ifaces, 0);
    assert!(rx.try_recv().is_err());
}

/// Default is `True`, so the opt-out is explicit and an unset interface carries it.
#[tokio::test]
async fn announces_from_internal_unset_allows_a_remote_announce_from_an_internal_next_hop() {
    let mut mgr = InterfaceManager::new(16);
    let mut rx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Unicast, InterfaceMode::Full)
        .tx_channel;
    let packet = announce_packet();
    let trace = mgr
        .send_with_announce_policy(
            TxMessage { tx_type: TxMessageType::Broadcast(None), packet: packet.clone() },
            Some(AnnounceBroadcastPolicy {
                local_destination: false,
                next_hop_iface_mode: Some(InterfaceMode::Internal),
                next_hop_announces_to_internal: None,
            }),
        )
        .await;
    assert_eq!(trace.sent_ifaces, 1);
    assert_eq!(rx.try_recv().expect("announce").packet, packet);
}

/// The opt-out is scoped to an internal next hop; any other next hop is
/// unaffected by it.
#[tokio::test]
async fn announces_from_internal_false_still_allows_a_remote_announce_from_a_full_next_hop() {
    let mut mgr = InterfaceManager::new(16);
    let mut rx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Unicast, InterfaceMode::Full)
        .tx_channel;
    let iface = mgr.ifaces[0].address;
    mgr.set_shared_config(
        iface,
        InterfaceSharedConfig { announces_from_internal: Some(false), ..Default::default() },
    );
    let packet = announce_packet();
    let trace = mgr
        .send_with_announce_policy(
            TxMessage { tx_type: TxMessageType::Broadcast(None), packet: packet.clone() },
            Some(AnnounceBroadcastPolicy {
                local_destination: false,
                next_hop_iface_mode: Some(InterfaceMode::Full),
                next_hop_announces_to_internal: None,
            }),
        )
        .await;
    assert_eq!(trace.sent_ifaces, 1);
    assert_eq!(rx.try_recv().expect("announce").packet, packet);
}

/// ...and scoped to remote announces: this node's own destination still
/// announces over an opted-out interface.
#[tokio::test]
async fn announces_from_internal_false_still_allows_our_own_announce() {
    let mut mgr = InterfaceManager::new(16);
    let mut rx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Unicast, InterfaceMode::Full)
        .tx_channel;
    let iface = mgr.ifaces[0].address;
    mgr.set_shared_config(
        iface,
        InterfaceSharedConfig { announces_from_internal: Some(false), ..Default::default() },
    );
    let packet = announce_packet();
    let trace = mgr
        .send_with_announce_policy(
            TxMessage { tx_type: TxMessageType::Broadcast(None), packet: packet.clone() },
            Some(AnnounceBroadcastPolicy {
                local_destination: true,
                next_hop_iface_mode: Some(InterfaceMode::Internal),
                next_hop_announces_to_internal: None,
            }),
        )
        .await;
    assert_eq!(trace.sent_ifaces, 1);
    assert_eq!(rx.try_recv().expect("announce").packet, packet);
}

/// The opt-out precedes the mode ladder, so it applies even to a Gateway.
#[tokio::test]
async fn announces_from_internal_false_blocks_on_a_gateway_interface() {
    let mut mgr = InterfaceManager::new(16);
    let mut rx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Unicast, InterfaceMode::Gateway)
        .tx_channel;
    let iface = mgr.ifaces[0].address;
    mgr.set_shared_config(
        iface,
        InterfaceSharedConfig { announces_from_internal: Some(false), ..Default::default() },
    );
    let packet = announce_packet();
    let trace = mgr
        .send_with_announce_policy(
            TxMessage { tx_type: TxMessageType::Broadcast(None), packet: packet.clone() },
            Some(AnnounceBroadcastPolicy {
                local_destination: false,
                next_hop_iface_mode: Some(InterfaceMode::Internal),
                next_hop_announces_to_internal: None,
            }),
        )
        .await;
    assert_eq!(trace.sent_ifaces, 0);
    assert!(rx.try_recv().is_err());
}

/// An internal-mode interface refuses an announce that reached this node over
/// a boundary — a boundary marks the edge of a local topology.
#[tokio::test]
async fn internal_blocks_a_remote_announce_from_a_boundary_next_hop() {
    let mut mgr = InterfaceManager::new(16);
    let mut rx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Unicast, InterfaceMode::Internal)
        .tx_channel;
    let packet = announce_packet();
    let trace = mgr
        .send_with_announce_policy(
            TxMessage { tx_type: TxMessageType::Broadcast(None), packet: packet.clone() },
            Some(AnnounceBroadcastPolicy {
                local_destination: false,
                next_hop_iface_mode: Some(InterfaceMode::Boundary),
                next_hop_announces_to_internal: None,
            }),
        )
        .await;
    assert_eq!(trace.sent_ifaces, 0);
    assert!(rx.try_recv().is_err());
}

/// ...unless that boundary interface sets `announces_to_internal`, which is
/// the reference's explicit override.
#[tokio::test]
async fn internal_allows_a_boundary_next_hop_that_announces_to_internal() {
    let mut mgr = InterfaceManager::new(16);
    let mut rx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Unicast, InterfaceMode::Internal)
        .tx_channel;
    let packet = announce_packet();
    let trace = mgr
        .send_with_announce_policy(
            TxMessage { tx_type: TxMessageType::Broadcast(None), packet: packet.clone() },
            Some(AnnounceBroadcastPolicy {
                local_destination: false,
                next_hop_iface_mode: Some(InterfaceMode::Boundary),
                next_hop_announces_to_internal: Some(true),
            }),
        )
        .await;
    assert_eq!(trace.sent_ifaces, 1);
    assert_eq!(rx.try_recv().expect("announce").packet, packet);
}

/// Every other next-hop mode crosses onto an internal interface normally.
#[tokio::test]
async fn internal_allows_a_remote_announce_from_a_full_next_hop() {
    let mut mgr = InterfaceManager::new(16);
    let mut rx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Unicast, InterfaceMode::Internal)
        .tx_channel;
    let packet = announce_packet();
    let trace = mgr
        .send_with_announce_policy(
            TxMessage { tx_type: TxMessageType::Broadcast(None), packet: packet.clone() },
            Some(AnnounceBroadcastPolicy {
                local_destination: false,
                next_hop_iface_mode: Some(InterfaceMode::Full),
                next_hop_announces_to_internal: None,
            }),
        )
        .await;
    assert_eq!(trace.sent_ifaces, 1);
    assert_eq!(rx.try_recv().expect("announce").packet, packet);
}

/// A locally-owned destination always announces onto an internal interface.
#[tokio::test]
async fn internal_allows_our_own_announce_over_a_boundary_next_hop() {
    let mut mgr = InterfaceManager::new(16);
    let mut rx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Unicast, InterfaceMode::Internal)
        .tx_channel;
    let packet = announce_packet();
    let trace = mgr
        .send_with_announce_policy(
            TxMessage { tx_type: TxMessageType::Broadcast(None), packet: packet.clone() },
            Some(AnnounceBroadcastPolicy {
                local_destination: true,
                next_hop_iface_mode: Some(InterfaceMode::Boundary),
                next_hop_announces_to_internal: None,
            }),
        )
        .await;
    assert_eq!(trace.sent_ifaces, 1);
    assert_eq!(rx.try_recv().expect("announce").packet, packet);
}

/// A virtual child copies the host's shared config when it is registered, and
/// `for_next_hop` reads the policy off whichever interface the path table
/// recorded as the next hop — for a discovered peer that is the child. A policy
/// applied to a live host therefore has to reach the children already on it.
#[test]
fn set_shared_config_reaches_children_registered_before_the_change() {
    let mut mgr = InterfaceManager::new(16);
    let _host_tx = mgr
        .new_channel_with_role_and_mode(16, IfaceRole::Multicast, InterfaceMode::Boundary)
        .tx_channel;
    let host = mgr.ifaces[0].address;
    let peer = mgr.register_virtual_iface(host, IfaceRole::VirtualUnicast).expect("virtual iface");

    assert!(mgr.set_shared_config(
        host,
        InterfaceSharedConfig { announces_to_internal: Some(true), ..Default::default() },
    ));

    assert_eq!(
        AnnounceBroadcastPolicy::for_next_hop(&mgr, Some(peer), false)
            .next_hop_announces_to_internal,
        Some(true),
        "a discovered peer must carry the applied policy without being recreated"
    );
}
