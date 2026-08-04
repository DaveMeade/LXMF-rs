/// A request carries a whole window, not the one slot a part just vacated.
///
/// Measured over a real 46 MB transfer before this: 16,919 requests for
/// 39,809 fragments, 15,689 of them carrying exactly one hash. That is a
/// request packet and a round trip per fragment, which makes the window
/// ladder meaningless in bandwidth terms however high it climbs.
#[test]
fn a_request_asks_for_the_whole_window_at_once() {
    let (mut receiver, _) = multi_segment_receiver(600, 600);
    let now = Instant::now();
    let rtt = Duration::from_millis(50);

    receiver.window = 32;
    let request =
        receiver.build_request(now, rtt, TEST_ARRIVAL_INTERVAL, RequestTrigger::Immediate);
    assert_eq!(
        request.requested_hashes.len(),
        32,
        "one request should carry the whole window: {}",
        request.requested_hashes.len()
    );
}

/// While a round is still in flight, a newly arrived part must not trigger
/// another request. The reference only re-requests on the receive path once
/// `outstanding_parts` reaches zero (`RNS/Resource.py`).
#[test]
fn an_arriving_part_does_not_re_request_until_the_round_drains() {
    let (mut receiver, _) = multi_segment_receiver(600, 600);
    let now = Instant::now();
    let rtt = Duration::from_millis(50);

    receiver.window = 16;
    let opening =
        receiver.build_request(now, rtt, TEST_ARRIVAL_INTERVAL, RequestTrigger::Immediate);
    assert_eq!(opening.requested_hashes.len(), 16);

    // One fragment of the sixteen lands.
    receiver.parts[0] = Some(b"body".to_vec());
    receiver.received += 1;
    receiver.in_flight_set.remove(&0);

    let mid_round =
        receiver.build_request(now, rtt, TEST_ARRIVAL_INTERVAL, RequestTrigger::PartReceived);
    assert!(
        mid_round.requested_hashes.is_empty(),
        "asking here would carry exactly the one slot that part vacated: {:?}",
        mid_round.requested_hashes.len()
    );

    // The rest of the round lands: now a request is due, and it is a full one.
    for idx in 1..16 {
        receiver.parts[idx] = Some(b"body".to_vec());
        receiver.received += 1;
        receiver.in_flight_set.remove(&idx);
    }
    receiver.consecutive_completed_height = 16;
    let next =
        receiver.build_request(now, rtt, TEST_ARRIVAL_INTERVAL, RequestTrigger::PartReceived);
    assert_eq!(next.requested_hashes.len(), 16, "a drained round refills the whole window");
}

/// The idle budget has to scale with the outstanding window, because a sender
/// serves fragments in sequence. A fixed budget makes a large window declare
/// its own tail lost on a perfect link, and every such "loss" ratchets the
/// ceiling down permanently — `note_fragments_lost` has no inverse.
#[test]
fn the_loss_budget_grows_with_the_window_it_is_waiting_on() {
    let (mut receiver, _) = multi_segment_receiver(600, 600);
    let now = Instant::now();
    let rtt = Duration::from_millis(41);
    let arrival = Duration::from_millis(2);

    receiver.window = 4;
    receiver.build_request(now, rtt, arrival, RequestTrigger::Immediate);
    let small = receiver.part_timeout(rtt, arrival);

    let (mut wide, _) = multi_segment_receiver(600, 600);
    wide.window = 60;
    wide.build_request(now, rtt, arrival, RequestTrigger::Immediate);
    let large = wide.part_timeout(rtt, arrival);

    assert!(
        large > small,
        "a window of 60 needs longer than a window of 4: {large:?} vs {small:?}"
    );
    // 60 fragments at a 2 ms measured interval is 120 ms of service time
    // alone, which the old fixed 2×rtt (82 ms) could not cover.
    assert!(large > rtt * 2, "the budget must exceed the old fixed 2×rtt: {large:?}");
}

/// Even with nothing measured yet, the budget never collapses toward zero.
/// On a fast link the measured interval is a fraction of a millisecond, and a
/// purely rate-derived budget would declare loss almost immediately.
#[test]
fn the_loss_budget_has_an_absolute_floor() {
    let (receiver, _) = multi_segment_receiver(600, 600);
    let floor = receiver.part_timeout(Duration::ZERO, Duration::ZERO);
    assert!(
        floor >= Duration::from_millis(250),
        "a rate-derived budget with no floor collapses on a fast link: {floor:?}"
    );
}

/// A split transfer is many resources, and each one restarting at `WINDOW`
/// re-learns a link the previous segment just measured. A 46 MB file arrives
/// as 45 segments, so the ladder was paying that cost forty-five times. The
/// reference carries the window on the Link itself (`Link.resource_concluded`
/// stores it, `RNS/Resource.py` restores it).
#[test]
fn a_new_resource_starts_from_the_window_the_last_one_earned() {
    let signer = PrivateIdentity::new_from_rand(OsRng);
    let identity = *signer.as_identity();
    let destination = DestinationDesc {
        identity,
        address_hash: identity.address_hash,
        name: DestinationName::new("lxmf", "resource"),
    };
    let (tx, _) = tokio::sync::broadcast::channel(1);
    let mut link = Link::new(destination, tx);
    link.request();
    let mut manager = ResourceManager::new_with_config(Duration::from_secs(1), 2);

    // Stand in for a previous resource on this link having finished at 24.
    manager.link_stats.entry(*link.id()).or_insert_with(LinkStats::new).last_window = Some(24);

    let data = b"second-resource-on-the-same-link";
    let (mut adv, _part) = split_test_segment(data, None, 1, 1, data.len() as u64);
    adv.flags = 0;
    let packet = resource_packet(
        PacketContext::ResourceAdvrtisement,
        &adv.pack().expect("advertisement"),
        *link.id(),
    );
    assert_eq!(manager.handle_packet(&packet, &mut link).len(), 1);

    let receiver = manager.incoming.get(&adv.hash).expect("receiver for the new resource");
    assert_eq!(
        receiver.window, 24,
        "the second resource should not re-climb a ladder the first one already climbed"
    );
}

/// A carried window may legitimately exceed the new resource's ceiling. The
/// ceiling gates *growth*; making a link that has demonstrated 75 re-climb
/// from 10 would throw away the measurement this whole mechanism exists to
/// keep. The loss path must survive that ordering without underflowing.
#[test]
fn a_carried_window_may_start_above_the_new_resources_ceiling() {
    let signer = PrivateIdentity::new_from_rand(OsRng);
    let identity = *signer.as_identity();
    let destination = DestinationDesc {
        identity,
        address_hash: identity.address_hash,
        name: DestinationName::new("lxmf", "resource"),
    };
    let (tx, _) = tokio::sync::broadcast::channel(1);
    let mut link = Link::new(destination, tx);
    link.request();
    let mut manager = ResourceManager::new_with_config(Duration::from_secs(1), 2);

    manager.link_stats.entry(*link.id()).or_insert_with(LinkStats::new).last_window =
        Some(WINDOW_MAX_FAST);

    let data = b"a-resource-that-has-measured-nothing-yet";
    let (mut adv, _part) = split_test_segment(data, None, 1, 1, data.len() as u64);
    adv.flags = 0;
    let packet = resource_packet(
        PacketContext::ResourceAdvrtisement,
        &adv.pack().expect("advertisement"),
        *link.id(),
    );
    assert_eq!(manager.handle_packet(&packet, &mut link).len(), 1);

    let receiver = manager.incoming.get(&adv.hash).expect("receiver for the new resource");
    assert_eq!(receiver.window, WINDOW_MAX_FAST, "the earned window carries over intact");
    assert_eq!(
        receiver.window_max, WINDOW_MAX_SLOW,
        "but the ceiling still has to be re-earned by measurement"
    );

    // The loss path compares `window_max - window`, which is inverted here.
    let mut receiver = manager.incoming.remove(&adv.hash).expect("receiver");
    receiver.note_fragments_lost();
    assert_eq!(receiver.window, WINDOW_MAX_FAST - 1, "a loss narrows the carried window by one");
}
